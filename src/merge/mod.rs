mod behaviours;
mod execute;
mod log;
#[cfg(test)]
mod tests;

use crate::config::Config;
use crate::domain::{GhApiQueryParam, Repo};
use anyhow::Context;
pub use behaviours::RunBehaviours;
use chrono::Utc;
use execute::merge_pr_for_repo;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use log::RunLog;
use octocrab::Octocrab;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_FETCH_TASKS: usize = 50;

pub async fn merge_prs(
    client: Arc<Octocrab>,
    config: Config,
    repos_override: Vec<Repo>,
    behaviours: RunBehaviours,
) -> anyhow::Result<()> {
    let mut l = RunLog::new(std::io::stdout(), &behaviours);

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
    l.info(&format!("The time right now is {start}"));

    if let Some(b) = &config.base_branch {
        l.info(&format!(
            "I'm only looking for PRs where the base branch is \"{b}\""
        ));
    }

    if config.merge_if_blocked {
        l.info("I will merge PRs even if they're blocked");
    }

    if !config.merge_if_checks_skipped {
        l.info("I won't merge PRs if checks are skipped");
    }

    if behaviours.show_repos_with_no_prs {
        l.info("I will show repositories that have no PRs");
    }

    if behaviours.show_prs_from_untrusted_authors {
        l.info("I will show PRs from untrusted authors");
    }

    if behaviours.show_prs_with_unmatched_head && config.head_pattern.is_some() {
        l.info("I will show PRs from where head doesn't match configured head pattern");
    }

    l.info(&format!(
        r#"I'm sorting PRs based on "{}" in the "{}" direction"#,
        config.sort_by.readable_repr(),
        config.sort_direction.readable_repr()
    ));

    let config = Arc::new(config);

    let semaphore = Arc::new(Semaphore::new(MAX_FETCH_TASKS));
    let mut futures = FuturesUnordered::new();
    for repo in repos_to_use {
        let semaphore = Arc::clone(&semaphore);
        let client = Arc::clone(&client);
        let config = Arc::clone(&config);
        futures.push(tokio::task::spawn(async move {
            merge_pr_for_repo(semaphore, client, config, repo.clone(), behaviours.execute).await
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
        "This run ended at {end_ts}; took {num_seconds} seconds"
    ));

    l.write_output().context("couldn't write output to file")?;

    Ok(())
}
