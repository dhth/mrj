use crate::config::Config;
use crate::domain::{
    Disqualification, MergeResult, Repo, RepoCheck, RepoCheckFinished, RepoResult, RunMergeResults,
    RunSummary,
};
use crate::merge::RunBehaviours;
use crate::merge::log::RunLogger;
use crate::merge::process::merge_pr_for_repo;
use anyhow::Context;
use chrono::Utc;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use octocrab::Octocrab;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_FETCH_TASKS: usize = 50;

pub(crate) async fn merge_prs(
    client: Arc<Octocrab>,
    config: Arc<Config>,
    repos_override: Vec<Repo>,
    behaviours: RunBehaviours,
) -> anyhow::Result<Option<RunMergeResults>> {
    let mut logger = RunLogger::new(std::io::stdout(), &behaviours);
    let mut results = vec![];

    let repos_to_use = if repos_override.is_empty() {
        config.repos.clone()
    } else {
        repos_override
    };

    if repos_to_use.is_empty() {
        return Ok(None);
    }

    let started_at = Utc::now();
    logger.print_banner();
    logger.print_startup_info(config.as_ref(), started_at);

    let semaphore = Arc::new(Semaphore::new(MAX_FETCH_TASKS));
    let mut futures = FuturesUnordered::new();
    for repo in repos_to_use {
        let semaphore = Arc::clone(&semaphore);
        let client = Arc::clone(&client);
        let config = Arc::clone(&config);
        futures.push(tokio::task::spawn(async move {
            merge_pr_for_repo(semaphore, client, config.as_ref(), repo, behaviours.execute).await
        }));
    }

    while let Some(result) = futures.next().await {
        let result = result.context("couldn't join merge task")?;

        if let Some(result) = filter_repo_result(result, &behaviours) {
            logger.add_repo_result(&result);
            results.push(result);
        }
    }
    let summary = RunSummary::from_results(&results, behaviours.execute);

    let ended_at = Utc::now();
    let num_seconds = (ended_at - started_at).num_seconds();
    logger.print_conclusion(ended_at, num_seconds);

    logger
        .write_output(&summary)
        .context("couldn't write output to file")?;

    Ok(Some(RunMergeResults {
        results,
        summary,
        started_at,
        ended_at,
    }))
}

fn filter_repo_result(result: RepoResult, behaviours: &RunBehaviours) -> Option<RepoResult> {
    match result {
        RepoResult::Errored(repo_check) => Some(RepoResult::Errored(repo_check)),
        RepoResult::Finished(RepoCheck {
            owner,
            name,
            state: RepoCheckFinished(results),
        }) => {
            let filtered_results = results
                .into_iter()
                .filter(|result| should_show_merge_result(result, behaviours))
                .collect::<Vec<_>>();

            if filtered_results.is_empty() && !behaviours.show_repos_with_no_prs {
                return None;
            }

            Some(RepoResult::Finished(RepoCheck {
                owner,
                name,
                state: RepoCheckFinished(filtered_results),
            }))
        }
    }
}

fn should_show_merge_result(result: &MergeResult, behaviours: &RunBehaviours) -> bool {
    match result {
        MergeResult::Disqualified(pr_check) => match pr_check.state.reason() {
            Disqualification::Author(_) if !behaviours.show_prs_from_untrusted_authors => false,
            Disqualification::Head(_) if !behaviours.show_prs_with_unmatched_head => false,
            _ => true,
        },
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PRCheck, PRCheckErrored, PRCheckFinished, PRDisqualified, Qualification};
    use insta::assert_yaml_snapshot;

    const OWNER: &str = "dhth";
    const REPO: &str = "mrj";
    const PR_TITLE: &str = "build: bump clap from 4.5.39 to 4.5.40";
    const PR_URL: &str = "https://github.com/dhth/mrj/pull/1";
    const PR_HEAD: &str = "dependabot/cargo/clap-4.5.40";

    #[test]
    fn filtering_repo_result_keeps_qualified_prs() {
        // GIVEN
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_qualified()]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &RunBehaviours::default());

        // THEN
        assert_yaml_snapshot!(filtered_result, @r#"
        Finished:
          owner: dhth
          name: mrj
          state:
            - Qualified:
                number: 3
                title: "build: bump clap from 4.5.39 to 4.5.40"
                url: "https://github.com/dhth/mrj/pull/1"
                pr_created_at: ~
                pr_updated_at: ~
                qualifications:
                  - Head: dependabot/cargo/clap-4.5.40
                state: ~
        "#
        );
    }

    #[test]
    fn filtering_repo_result_passes_through_errored_repo_results() {
        // GIVEN
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_errored()]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &RunBehaviours::default());

        // THEN
        assert_yaml_snapshot!(filtered_result, @r#"
        Finished:
          owner: dhth
          name: mrj
          state:
            - Errored:
                number: 4
                title: "build: bump clap from 4.5.39 to 4.5.40"
                url: "https://github.com/dhth/mrj/pull/1"
                pr_created_at: ~
                pr_updated_at: ~
                qualifications:
                  - Head: dependabot/cargo/clap-4.5.40
                state: "couldn't merge PR: GitHub API was down"
        "#
        );
    }

    #[test]
    fn filtering_repo_result_filters_out_head_disqualified_prs_by_default() {
        // GIVEN
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_unmatched_head()]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &RunBehaviours::default());

        // THEN
        assert!(filtered_result.is_none());
    }

    #[test]
    fn filtering_repo_result_filters_out_author_disqualified_prs_by_default() {
        // GIVEN
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_untrusted_author()]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &RunBehaviours::default());

        // THEN
        assert!(filtered_result.is_none());
    }

    #[test]
    fn filtering_repo_result_keeps_head_disqualified_prs_when_requested() {
        // GIVEN
        let behaviours = RunBehaviours::default().show_prs_with_unmatched_head();
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_unmatched_head()]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &behaviours);

        // THEN
        assert_yaml_snapshot!(filtered_result, @r#"
        Finished:
          owner: dhth
          name: mrj
          state:
            - Disqualified:
                number: 1
                title: "build: bump clap from 4.5.39 to 4.5.40"
                url: "https://github.com/dhth/mrj/pull/1"
                pr_created_at: ~
                pr_updated_at: ~
                qualifications: []
                state:
                  Head: big-refactor
        "#
        );
    }

    #[test]
    fn filtering_repo_result_keeps_author_disqualified_prs_when_requested() {
        // GIVEN
        let behaviours = RunBehaviours::default().show_prs_from_untrusted_authors();
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_untrusted_author()]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &behaviours);

        // THEN
        assert_yaml_snapshot!(filtered_result, @r#"
        Finished:
          owner: dhth
          name: mrj
          state:
            - Disqualified:
                number: 2
                title: "build: bump clap from 4.5.39 to 4.5.40"
                url: "https://github.com/dhth/mrj/pull/1"
                pr_created_at: ~
                pr_updated_at: ~
                qualifications:
                  - Head: dependabot/cargo/clap-4.5.40
                state:
                  Author: untrusted-author
        "#
        );
    }

    #[test]
    fn filtering_repo_result_keeps_repo_with_empty_results_when_requested() {
        // GIVEN
        let behaviours = RunBehaviours {
            show_repos_with_no_prs: true,
            ..RunBehaviours::default()
        };
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_unmatched_head()]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &behaviours);

        // THEN
        assert_yaml_snapshot!(filtered_result, @"
        Finished:
          owner: dhth
          name: mrj
          state: []
        "
        );
    }

    #[test]
    fn filtering_repo_result_keeps_only_visible_results_in_mixed_repo_results() {
        // GIVEN
        let result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![
                merge_result_disqualified_unmatched_head(),
                merge_result_disqualified_untrusted_author(),
                merge_result_errored(),
                merge_result_qualified(),
            ]),
        });

        // WHEN
        let filtered_result = filter_repo_result(result, &RunBehaviours::default());

        // THEN
        assert_yaml_snapshot!(filtered_result, @r#"
        Finished:
          owner: dhth
          name: mrj
          state:
            - Errored:
                number: 4
                title: "build: bump clap from 4.5.39 to 4.5.40"
                url: "https://github.com/dhth/mrj/pull/1"
                pr_created_at: ~
                pr_updated_at: ~
                qualifications:
                  - Head: dependabot/cargo/clap-4.5.40
                state: "couldn't merge PR: GitHub API was down"
            - Qualified:
                number: 3
                title: "build: bump clap from 4.5.39 to 4.5.40"
                url: "https://github.com/dhth/mrj/pull/1"
                pr_created_at: ~
                pr_updated_at: ~
                qualifications:
                  - Head: dependabot/cargo/clap-4.5.40
                state: ~
        "#
        );
    }

    fn merge_result_disqualified_unmatched_head() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![],
            state: PRDisqualified(Disqualification::Head("big-refactor".to_string())),
        })
    }

    fn merge_result_disqualified_untrusted_author() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 2,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::Author(Some(
                "untrusted-author".to_string(),
            ))),
        })
    }

    fn merge_result_qualified() -> MergeResult {
        MergeResult::Qualified(PRCheck {
            number: 3,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRCheckFinished,
        })
    }

    fn merge_result_errored() -> MergeResult {
        MergeResult::Errored(PRCheck {
            number: 4,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRCheckErrored(anyhow::anyhow!("couldn't merge PR: GitHub API was down")),
        })
    }
}
