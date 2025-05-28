use crate::config::Config;
use crate::domain::{Disqualification as DQ, PRResult, Qualification as Q, Repo, RepoResult};
use anyhow::Context;
use octocrab::Octocrab;
use octocrab::{
    models::pulls::{MergeableState, PullRequest},
    params::{State, repos::Commitish},
};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn merge_pr_for_repo(
    semaphore: Arc<Semaphore>,
    client: Arc<Octocrab>,
    config: Arc<Config>,
    repo: Repo,
    dry_run: bool,
) -> RepoResult {
    let mut result = RepoResult::new(&repo.owner, &repo.repo);

    // acquiring of the semaphore permit is done inside this function
    // so that an error, if any, can be reported in the log
    match semaphore
        .acquire()
        .await
        .context("couldn't acquire semaphore permit")
    {
        Ok(_) => {}
        Err(err) => {
            return result.record_error(err);
        }
    }

    let pulls = client.pulls(&repo.owner, &repo.repo);

    let mut page_builder = pulls.list().state(State::Open).per_page(100);

    if let Some(base_branch) = &config.base_branch {
        page_builder = page_builder.base(base_branch);
    }

    let page = match page_builder.send().await.context("couldn't get PRs") {
        Ok(p) => p,
        Err(err) => {
            return result.record_error(err);
        }
    };

    if page.items.is_empty() {
        return result;
    }

    // This cannot be run concurrently as we only want to merge 1 PR for a
    // repo in a run
    for pull_request in &page {
        let pr_result = merge_pr(
            &repo.owner,
            &repo.repo,
            pull_request,
            client.as_ref(),
            config.as_ref(),
            dry_run,
        )
        .await;
        let no_failure = pr_result.failure.is_none();
        result.add_pr_result(pr_result);

        if !dry_run && no_failure {
            // PR was merged
            break;
        }
    }

    result
}

async fn merge_pr(
    owner: &str,
    repo: &str,
    pull_request: &PullRequest,
    client: &Octocrab,
    config: &Config,
    dry_run: bool,
) -> PRResult {
    let mut result = PRResult::new(
        pull_request.number,
        &pull_request.title.clone().unwrap_or_default(),
        &pull_request
            .html_url
            .as_ref()
            .map(|url| url.to_string())
            .unwrap_or_default(),
    );

    if let Some(head_pattern) = &config.head_pattern {
        let head_ref = pull_request.head.ref_field.clone();
        if head_pattern.re.is_match(&head_ref) {
            result.add_qualification(Q::Head(head_ref));
        } else {
            return result.disqualify(DQ::Head(head_ref));
        }
    }

    match &pull_request.user {
        Some(trusted_user) if config.trusted_authors.contains(&trusted_user.login) => {
            result.add_qualification(Q::User(trusted_user.login.clone()));
        }
        Some(other_user) => {
            return result.disqualify(DQ::User(Some(other_user.login.clone())));
        }
        None => {
            return result.disqualify(DQ::User(None));
        }
    }

    let pr = match client
        .pulls(owner, repo)
        .get(pull_request.number)
        .await
        .context("couldn't get details")
    {
        Ok(pr) => pr,
        Err(err) => {
            return result.record_error(err);
        }
    };

    let pr_head_ref = pr.head.sha;

    let checks = match client
        .checks(owner, repo)
        .list_check_runs_for_git_ref(Commitish::from(pr_head_ref.clone()))
        .send()
        .await
        .context("couldn't get pr checks")
    {
        Ok(c) => c,
        Err(err) => {
            return result.record_error(err);
        }
    };

    for check in &checks.check_runs {
        match check.conclusion.as_deref() {
            Some("success") => {
                result.add_qualification(Q::Check {
                    name: check.name.clone(),
                    conclusion: "success".to_string(),
                });
            }
            Some("skipped") => {
                if config.merge_if_checks_skipped {
                    result.add_qualification(Q::Check {
                        name: check.name.clone(),
                        conclusion: "success".to_string(),
                    });
                } else {
                    return result.disqualify(DQ::Check {
                        name: check.name.clone(),
                        conclusion: Some("skipped".to_string()),
                    });
                }
            }
            Some(non_successful_conclusion) => {
                return result.disqualify(DQ::Check {
                    name: check.name.clone(),
                    conclusion: Some(non_successful_conclusion.to_string()),
                });
            }
            None => {
                return result.disqualify(DQ::Check {
                    name: check.name.clone(),
                    conclusion: None,
                });
            }
        }
    }

    match pr.mergeable_state.as_ref() {
        Some(state) => match state {
            MergeableState::Clean => {
                result.add_qualification(Q::State("clean".to_string()));
            }
            MergeableState::Blocked if config.merge_if_blocked => {
                result.add_qualification(Q::State("blocked".to_string()));
            }
            other => {
                return result.disqualify(DQ::State(Some(format!("{:?}", other))));
            }
        },
        None => {
            return result.disqualify(DQ::State(None));
        }
    }

    if !dry_run {
        if let Err(err) = client
            .pulls(owner, repo)
            .merge(pr.number)
            .method(config.merge_type.merge_method())
            .send()
            .await
            .context("couldn't merge PR")
        {
            return result.record_error(err);
        }
    }

    result
}
