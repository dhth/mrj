mod log;

use crate::config::Config;
use crate::domain::{Disqualification as DQ, Qualification as Q, Repo};
use anyhow::Context;
use chrono::Utc;
use log::RunLog;
use octocrab::Octocrab;
use octocrab::{
    models::pulls::MergeableState,
    params::{State, repos::Commitish},
};
use std::path::Path;

pub async fn merge_prs<P>(
    client: Octocrab,
    config: Config,
    repos_override: Vec<Repo>,
    output: bool,
    output_file: P,
    dry_run: bool,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut l = RunLog::new(output);

    let repos_to_use = if repos_override.is_empty() {
        &config.repos
    } else {
        &repos_override
    };

    l.banner(dry_run);

    let start = Utc::now();
    l.info(&format!("The time right now is {}", start));

    if let Some(b) = &config.base_branch {
        l.info(&format!(
            "I'm only looking for PRs where the base branch is \"{}\"",
            b
        ));
    }

    if config.merge_if_blocked {
        l.info("I will merge PRs even if they're blocked");
    }

    if !config.merge_if_checks_skipped {
        l.info("I wont merge PRs if checks are skipped");
    }

    for repo in repos_to_use {
        l.repo_info(&repo.repo);

        let pulls = client.pulls(&repo.owner, &repo.repo);

        let mut page_builder = pulls.list().state(State::Open).per_page(100);

        if let Some(base_branch) = &config.base_branch {
            page_builder = page_builder.base(base_branch);
        }

        let page = match page_builder.send().await.context("couldn't get PRs") {
            Ok(p) => p,
            Err(err) => {
                l.error(err);
                continue;
            }
        };

        if page.items.is_empty() {
            l.empty_line();
            l.absence("no PRs");
            continue;
        }

        for pull_request in &page {
            l.pr_info(&format!(
                r#"
-> checking PR #{}
        {}
        {}"#,
                pull_request.number,
                pull_request.title.clone().unwrap_or_default(),
                pull_request
                    .html_url
                    .as_ref()
                    .map(|url| url.to_string())
                    .unwrap_or_default(),
            ));

            if let Some(head_pattern) = &config.head_pattern {
                let head_ref = pull_request.head.ref_field.clone();
                if head_pattern.re.is_match(&head_ref) {
                    l.qualification(Q::Head(head_ref));
                } else {
                    l.disqualification(DQ::Head(head_ref));
                    continue;
                }
            }

            match &pull_request.user {
                Some(trusted_user) if config.trusted_authors.contains(&trusted_user.login) => {
                    l.qualification(Q::User(trusted_user.login.clone()));
                }
                Some(other_user) => {
                    l.disqualification(DQ::User(Some(other_user.login.clone())));
                    continue;
                }
                None => {
                    l.disqualification(DQ::User(None));
                    continue;
                }
            }

            let pr = match client
                .pulls(&repo.owner, &repo.repo)
                .get(pull_request.number)
                .await
                .context("couldn't get details")
            {
                Ok(pr) => pr,
                Err(err) => {
                    l.error(err);
                    continue;
                }
            };

            let pr_head_ref = pr.head.sha;

            let checks = match client
                .checks(&repo.owner, &repo.repo)
                .list_check_runs_for_git_ref(Commitish::from(pr_head_ref.clone()))
                .send()
                .await
                .context("couldn't get pr checks")
            {
                Ok(c) => c,
                Err(err) => {
                    l.error(err);
                    continue;
                }
            };

            let mut skip = false;
            for check in &checks.check_runs {
                match check.conclusion.as_deref() {
                    Some("success") => {
                        l.qualification(Q::Check {
                            name: check.name.clone(),
                            conclusion: "success".to_string(),
                        });
                    }
                    Some("skipped") => {
                        if config.merge_if_checks_skipped {
                            l.qualification(Q::Check {
                                name: check.name.clone(),
                                conclusion: "success".to_string(),
                            });
                        } else {
                            l.disqualification(DQ::Check {
                                name: check.name.clone(),
                                conclusion: Some("skipped".to_string()),
                            });
                            skip = true;
                            break;
                        }
                    }
                    Some(non_successful_conclusion) => {
                        l.disqualification(DQ::Check {
                            name: check.name.clone(),
                            conclusion: Some(non_successful_conclusion.to_string()),
                        });
                        skip = true;
                        break;
                    }
                    None => {
                        l.disqualification(DQ::Check {
                            name: check.name.clone(),
                            conclusion: None,
                        });
                        skip = true;
                        break;
                    }
                }
            }

            if skip {
                continue;
            }

            match pr.mergeable_state.as_ref() {
                Some(state) => match state {
                    MergeableState::Clean => {
                        l.qualification(Q::State("clean".to_string()));
                    }
                    MergeableState::Blocked if config.merge_if_blocked => {
                        l.qualification(Q::State("blocked".to_string()));
                    }
                    other => {
                        l.disqualification(DQ::State(Some(format!("{:?}", other))));
                        continue;
                    }
                },
                None => {
                    l.disqualification(DQ::State(None));
                    continue;
                }
            }

            if dry_run {
                l.merge(
                    "PR matches all criteria, I would've merged it if this weren't a dry run ✅",
                    dry_run,
                );
            } else {
                match client
                    .pulls(&repo.owner, &repo.repo)
                    .merge(pr.number)
                    .method(config.merge_type.merge_method())
                    .send()
                    .await
                    .context("couldn't merge PR")
                {
                    Ok(_) => {
                        l.merge("PR merged! 🎉 ✅", dry_run);
                    }
                    Err(err) => {
                        l.error(err);
                    }
                };
            }
        }
    }

    let end_ts = Utc::now();
    let num_seconds = (end_ts - start).num_seconds();

    l.empty_line();
    l.info(&format!(
        "This run ended at {}; took {} seconds",
        end_ts, num_seconds
    ));

    l.write_output(output, output_file)
        .context("couldn't write output to file")?;

    Ok(())
}
