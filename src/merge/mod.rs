mod execute;
mod log;

use crate::config::Config;
use crate::domain::Repo;
use anyhow::Context;
use chrono::Utc;
use execute::merge_pr_for_repo;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use log::RunLog;
use octocrab::Octocrab;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_FETCH_TASKS: usize = 50;

pub struct RunBehaviours<P: AsRef<Path>> {
    pub output: bool,
    pub output_path: P,
    pub summary: bool,
    pub summary_path: P,
    pub ignore_repos_with_no_prs: bool,
    pub show_prs_from_untrusted_authors: bool,
    pub dry_run: bool,
}

pub async fn merge_prs<P>(
    client: Arc<Octocrab>,
    config: Config,
    repos_override: Vec<Repo>,
    behaviours: RunBehaviours<P>,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut l = RunLog::new(
        behaviours.output,
        behaviours.ignore_repos_with_no_prs,
        behaviours.show_prs_from_untrusted_authors,
        behaviours.dry_run,
    );

    let repos_to_use = if repos_override.is_empty() {
        config.repos.clone()
    } else {
        repos_override
    };

    if repos_to_use.is_empty() {
        return Ok(());
    }

    l.banner();

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
        l.info("I won't merge PRs if checks are skipped");
    }

    if behaviours.ignore_repos_with_no_prs {
        l.info("I won't show repositories that have no PRs");
    }

    if behaviours.show_prs_from_untrusted_authors {
        l.info("I won't show PRs from untrusted authors");
    }

    let config = Arc::new(config);

    let semaphore = Arc::new(Semaphore::new(MAX_FETCH_TASKS));
    let mut futures = FuturesUnordered::new();
    for repo in repos_to_use {
        let semaphore = Arc::clone(&semaphore);
        let client = Arc::clone(&client);
        let config = Arc::clone(&config);
        futures.push(tokio::task::spawn(async move {
            merge_pr_for_repo(semaphore, client, config, repo.clone(), behaviours.dry_run).await
        }));
    }

    while let Some(result) = futures.next().await {
        let result = result.context("couldn't join merge task")?;
        l.add_repo_result(result);
    }

    let end_ts = Utc::now();
    let num_seconds = (end_ts - start).num_seconds();

    l.empty_line();
    l.info(&format!(
        "This run ended at {}; took {} seconds",
        end_ts, num_seconds
    ));

    l.write_output(
        behaviours.output,
        behaviours.output_path,
        behaviours.summary,
        behaviours.summary_path,
    )
    .context("couldn't write output to file")?;

    Ok(())
}
