mod behaviours;
mod execute;
mod log;
#[cfg(test)]
mod tests;

use crate::config::Config;
use crate::domain::Repo;
use anyhow::Context;
pub use behaviours::RunBehaviours;
use chrono::Utc;
use execute::merge_pr_for_repo;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use log::RunLogger;
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
    let mut logger = RunLogger::new(std::io::stdout(), &behaviours);

    let repos_to_use = if repos_override.is_empty() {
        config.repos.clone()
    } else {
        repos_override
    };

    if repos_to_use.is_empty() {
        return Ok(());
    }

    logger.print_banner();
    let start = Utc::now();
    logger.print_startup_info(&config, start);

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
        logger.add_repo_result(result);
    }

    let end_ts = Utc::now();
    let num_seconds = (end_ts - start).num_seconds();
    logger.print_conclusion(end_ts, num_seconds);

    logger
        .write_output()
        .context("couldn't write output to file")?;

    Ok(())
}
