use crate::domain::{MergeType, SortBy, SortDirection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredRunEnvelope {
    pub version: u16,
    pub run: StoredRunData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredRunData {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub took_ms: i64,
    pub mode: StoredRunMode,
    pub config: StoredRunConfig,
    pub summary: StoredRunSummary,
    pub repos: Vec<StoredRepoRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredRunSummary {
    pub num_disqualifications: usize,
    pub num_errors: u16,
    pub num_merged: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoredRunMode {
    DryRun,
    Execute,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredRunConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_pattern: Option<String>,
    pub merge_if_blocked: bool,
    pub merge_if_checks_skipped: bool,
    #[serde(default)]
    pub merge_if_checks_neutral: bool,
    pub merge_type: StoredMergeType,
    pub sort_by: StoredSortBy,
    pub sort_direction: StoredSortDirection,
    pub flags: StoredRunFlags,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoredMergeType {
    Merge,
    Squash,
    Rebase,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoredSortBy {
    Created,
    Updated,
    Popularity,
    LongRunning,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoredSortDirection {
    Asc,
    Desc,
}

impl From<&MergeType> for StoredMergeType {
    fn from(value: &MergeType) -> Self {
        match value {
            MergeType::Merge => StoredMergeType::Merge,
            MergeType::Squash => StoredMergeType::Squash,
            MergeType::Rebase => StoredMergeType::Rebase,
        }
    }
}

impl From<&SortBy> for StoredSortBy {
    fn from(value: &SortBy) -> Self {
        match value {
            SortBy::Created => StoredSortBy::Created,
            SortBy::Updated => StoredSortBy::Updated,
            SortBy::Popularity => StoredSortBy::Popularity,
            SortBy::LongRunning => StoredSortBy::LongRunning,
        }
    }
}

impl From<&SortDirection> for StoredSortDirection {
    fn from(value: &SortDirection) -> Self {
        match value {
            SortDirection::Ascending => StoredSortDirection::Asc,
            SortDirection::Descending => StoredSortDirection::Desc,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredRunFlags {
    pub show_repos_with_no_prs: bool,
    pub show_prs_from_untrusted_authors: bool,
    pub show_prs_with_unmatched_head: bool,
    pub skip_disqualifications_in_summary: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredRepoRecord {
    pub repo: String,
    pub owner: String,
    pub name: String,
    pub status: StoredRepoStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub prs: Vec<StoredPrRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoredRepoStatus {
    Finished,
    Errored,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredPrRecord {
    pub number: u64,
    pub title: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    pub status: StoredPrStatus,
    pub qualifications: Vec<StoredQualification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disqualification: Option<StoredDisqualification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub merged: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoredPrStatus {
    Qualified,
    Disqualified,
    Errored,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum StoredQualification {
    Head { value: String },
    Author { value: String },
    Check { name: String, conclusion: String },
    State { value: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum StoredDisqualification {
    Head {
        value: String,
    },
    Author {
        value: Option<String>,
    },
    Check {
        name: String,
        conclusion: Option<String>,
    },
    State {
        value: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_run_config_defaults_missing_merge_if_checks_neutral_to_false() -> anyhow::Result<()> {
        let config: StoredRunConfig = serde_json::from_str(
            r#"{
                "merge_if_blocked": false,
                "merge_if_checks_skipped": true,
                "merge_type": "squash",
                "sort_by": "created",
                "sort_direction": "asc",
                "flags": {
                    "show_repos_with_no_prs": false,
                    "show_prs_from_untrusted_authors": false,
                    "show_prs_with_unmatched_head": false,
                    "skip_disqualifications_in_summary": false
                }
            }"#,
        )?;

        assert!(!config.merge_if_checks_neutral);

        Ok(())
    }
}
