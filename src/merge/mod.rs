mod execute;
mod log;

use crate::config::Config;
use crate::domain::Repo;
use anyhow::Context;
use chrono::Utc;
use execute::merge_pr_for_repo;
use log::RunLog;
use octocrab::Octocrab;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_FETCH_TASKS: usize = 50;

pub async fn merge_prs<P>(
    client: Arc<Octocrab>,
    config: Config,
    repos_override: Vec<Repo>,
    output: bool,
    output_file: P,
    dry_run: bool,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut l = RunLog::new(output, dry_run);

    let repos_to_use = if repos_override.is_empty() {
        config.repos.clone()
    } else {
        repos_override
    };

    if repos_to_use.is_empty() {
        return Ok(());
    }

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
        l.info("I won't merge PRs if checks are skipped");
    }

    let config = Arc::new(config);
    let mut tasks = Vec::new();

    let semaphore = Arc::new(Semaphore::new(MAX_FETCH_TASKS));
    for repo in repos_to_use {
        let semaphore = Arc::clone(&semaphore);
        let client = Arc::clone(&client);
        let config = Arc::clone(&config);
        tasks.push(tokio::task::spawn(async move {
            merge_pr_for_repo(semaphore, client, config, repo.clone(), dry_run).await
        }));
    }

    for task in tasks {
        let result = task.await.context("couldn't join merge task")?;
        l.add_result(result);
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
