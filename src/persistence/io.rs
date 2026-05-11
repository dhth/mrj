use super::schema::{
    StoredDisqualification, StoredPrRecord, StoredPrStatus, StoredQualification, StoredRepoRecord,
    StoredRepoStatus, StoredRunConfig, StoredRunData, StoredRunEnvelope, StoredRunFlags,
    StoredRunMode, StoredRunSummary,
};
use crate::config::Config;
use crate::domain::{
    Disqualification, MergeResult, Qualification, RepoResult, RunMergeResults, RunSummary,
};
use crate::merge::RunBehaviours;
use anyhow::Context;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const RUN_OUTPUT_SCHEMA_VERSION: u16 = 1;

pub fn persist_run(
    results: RunMergeResults,
    config: &Config,
    behaviours: &RunBehaviours,
    output_path: &Path,
) -> anyhow::Result<()> {
    let RunMergeResults {
        results,
        summary,
        started_at,
        ended_at: finished_at,
    } = results;

    let persisted_run = StoredRunEnvelope {
        version: RUN_OUTPUT_SCHEMA_VERSION,
        run: StoredRunData {
            started_at,
            finished_at,
            took_ms: (finished_at - started_at).num_milliseconds(),
            mode: if behaviours.execute {
                StoredRunMode::Execute
            } else {
                StoredRunMode::DryRun
            },
            config: map_config(config, behaviours),
            summary: map_summary(summary),
            repos: results
                .into_iter()
                .map(|result| map_repo_result(result, behaviours.execute))
                .collect(),
        },
    };

    write_run(output_path, &persisted_run).with_context(|| {
        format!(
            "couldn't write persisted run to {}",
            output_path.to_string_lossy()
        )
    })?;

    Ok(())
}

fn map_summary(summary: RunSummary) -> StoredRunSummary {
    StoredRunSummary {
        num_disqualifications: summary.disqualifications.len(),
        num_errors: summary.num_errors,
        num_merged: summary.prs_merged.len(),
    }
}

fn map_config(config: &Config, behaviours: &RunBehaviours) -> StoredRunConfig {
    StoredRunConfig {
        base_branch: config.base_branch.clone(),
        head_pattern: config
            .head_pattern
            .as_ref()
            .map(|pattern| pattern.re.as_str().to_string()),
        merge_if_blocked: config.merge_if_blocked,
        merge_if_checks_skipped: config.merge_if_checks_skipped,
        merge_type: (&config.merge_type).into(),
        sort_by: (&config.sort_by).into(),
        sort_direction: (&config.sort_direction).into(),
        flags: StoredRunFlags {
            show_repos_with_no_prs: behaviours.show_repos_with_no_prs,
            show_prs_from_untrusted_authors: behaviours.show_prs_from_untrusted_authors,
            show_prs_with_unmatched_head: behaviours.show_prs_with_unmatched_head,
            skip_disqualifications_in_summary: behaviours.skip_disqualifications_in_summary,
        },
    }
}

fn map_repo_result(result: RepoResult, did_execute: bool) -> StoredRepoRecord {
    match result {
        RepoResult::Errored(repo_check) => StoredRepoRecord {
            repo: format!("{}/{}", repo_check.owner, repo_check.name),
            owner: repo_check.owner,
            name: repo_check.name,
            status: StoredRepoStatus::Errored,
            error: Some(format!("{:#}", repo_check.state.reason())),
            prs: vec![],
        },
        RepoResult::Finished(repo_check) => {
            let repo = format!("{}/{}", repo_check.owner, repo_check.name);
            StoredRepoRecord {
                repo,
                owner: repo_check.owner,
                name: repo_check.name,
                status: StoredRepoStatus::Finished,
                error: None,
                prs: repo_check
                    .state
                    .0
                    .into_iter()
                    .map(|result| map_pr_result(result, did_execute))
                    .collect(),
            }
        }
    }
}

fn map_pr_result(result: MergeResult, did_execute: bool) -> StoredPrRecord {
    match result {
        MergeResult::Qualified(pr_check) => StoredPrRecord {
            number: pr_check.number,
            title: pr_check.title,
            url: pr_check.url,
            created_at: pr_check.pr_created_at,
            updated_at: pr_check.pr_updated_at,
            status: StoredPrStatus::Qualified,
            qualifications: pr_check
                .qualifications
                .into_iter()
                .map(map_qualification)
                .collect(),
            disqualification: None,
            error: None,
            merged: did_execute,
        },
        MergeResult::Disqualified(pr_check) => StoredPrRecord {
            number: pr_check.number,
            title: pr_check.title,
            url: pr_check.url,
            created_at: pr_check.pr_created_at,
            updated_at: pr_check.pr_updated_at,
            status: StoredPrStatus::Disqualified,
            qualifications: pr_check
                .qualifications
                .into_iter()
                .map(map_qualification)
                .collect(),
            disqualification: Some(map_disqualification(pr_check.state.0)),
            error: None,
            merged: false,
        },
        MergeResult::Errored(pr_check) => StoredPrRecord {
            number: pr_check.number,
            title: pr_check.title,
            url: pr_check.url,
            created_at: pr_check.pr_created_at,
            updated_at: pr_check.pr_updated_at,
            status: StoredPrStatus::Errored,
            qualifications: pr_check
                .qualifications
                .into_iter()
                .map(map_qualification)
                .collect(),
            disqualification: None,
            error: Some(format!("{:#}", pr_check.state.reason())),
            merged: false,
        },
    }
}

fn map_qualification(qualification: Qualification) -> StoredQualification {
    match qualification {
        Qualification::Head(value) => StoredQualification::Head { value },
        Qualification::Author(value) => StoredQualification::Author { value },
        Qualification::Check { name, conclusion } => {
            StoredQualification::Check { name, conclusion }
        }
        Qualification::State(value) => StoredQualification::State { value },
    }
}

fn map_disqualification(disqualification: Disqualification) -> StoredDisqualification {
    match disqualification {
        Disqualification::Head(value) => StoredDisqualification::Head { value },
        Disqualification::Author(value) => StoredDisqualification::Author { value },
        Disqualification::Check { name, conclusion } => {
            StoredDisqualification::Check { name, conclusion }
        }
        Disqualification::State(value) => StoredDisqualification::State { value },
    }
}

fn write_run<P>(path: P, persisted_run: &StoredRunEnvelope) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let contents = serde_json::to_vec_pretty(persisted_run)
        .context("couldn't serialize persisted run to JSON")?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.write_all(&contents)?;
    file.write_all(b"\n")?;

    Ok(())
}
