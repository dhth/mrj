use chrono::{DateTime, Utc};
use octocrab::models::pulls::PullRequest;
use octocrab::params::Direction;
use octocrab::params::pulls::{MergeMethod, Sort};
use regex::Regex;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt::{self, Display};
use std::path::PathBuf;

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum MergeType {
    Merge,
    Squash,
    Rebase,
}

impl MergeType {
    pub fn merge_method(&self) -> MergeMethod {
        match self {
            MergeType::Merge => MergeMethod::Merge,
            MergeType::Squash => MergeMethod::Squash,
            MergeType::Rebase => MergeMethod::Rebase,
        }
    }
}

impl<'de> Deserialize<'de> for MergeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MergeTypeVisitor;

        impl Visitor<'_> for MergeTypeVisitor {
            type Value = MergeType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(r#"either "merge" or "squash" or "rebase""#)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "merge" => Ok(MergeType::Merge),
                    "squash" => Ok(MergeType::Squash),
                    "rebase" => Ok(MergeType::Rebase),
                    _ => Err(de::Error::invalid_value(de::Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_str(MergeTypeVisitor)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct Repo {
    pub owner: String,
    pub repo: String,
}

impl Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

impl TryFrom<&str> for Repo {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.split_once("/") {
            Some((owner, repo)) => Ok(Repo {
                owner: owner.to_string(),
                repo: repo.to_string(),
            }),
            None => Err("repo needs to be in the form \"owner/repo\"".into()),
        }
    }
}

impl<'de> Deserialize<'de> for Repo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RepoVisitor;

        impl Visitor<'_> for RepoVisitor {
            type Value = Repo;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(r#"a value in the form "owner/repo""#)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value.split_once("/") {
                    Some((owner, repo)) => Ok(Repo {
                        owner: owner.to_string(),
                        repo: repo.to_string(),
                    }),
                    None => Err(de::Error::invalid_value(de::Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_str(RepoVisitor)
    }
}

#[derive(Debug)]
pub struct HeadPattern {
    pub re: Regex,
}

impl<'de> Deserialize<'de> for HeadPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HeadPatternVisitor;

        impl Visitor<'_> for HeadPatternVisitor {
            type Value = HeadPattern;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(r#"a valid regex"#)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match Regex::new(value) {
                    Ok(re) => Ok(HeadPattern { re }),
                    _ => Err(de::Error::invalid_value(de::Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_str(HeadPatternVisitor)
    }
}

pub trait GhApiQueryParam<T> {
    fn to_gh_api(&self) -> T;
    fn readable_repr(&self) -> &str;
}

#[derive(Debug, Deserialize, PartialEq)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "kebab-case")]
pub enum SortBy {
    Created,
    Updated,
    Popularity,
    LongRunning,
}

impl GhApiQueryParam<Sort> for SortBy {
    fn to_gh_api(&self) -> Sort {
        match self {
            SortBy::Created => Sort::Created,
            SortBy::Updated => Sort::Updated,
            SortBy::Popularity => Sort::Popularity,
            SortBy::LongRunning => Sort::LongRunning,
        }
    }

    fn readable_repr(&self) -> &str {
        match self {
            SortBy::Created => "creation date",
            SortBy::Updated => "last updated date",
            SortBy::Popularity => "popularity",
            SortBy::LongRunning => "long running status",
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

impl GhApiQueryParam<Direction> for SortDirection {
    fn to_gh_api(&self) -> Direction {
        match self {
            SortDirection::Ascending => Direction::Ascending,
            SortDirection::Descending => Direction::Descending,
        }
    }

    fn readable_repr(&self) -> &str {
        match self {
            SortDirection::Ascending => "ascending",
            SortDirection::Descending => "descending",
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum RepoResult {
    Finished(RepoCheck<RepoCheckFinished>),
    Errored(RepoCheck<RepoCheckErrored>),
}

impl RepoResult {
    pub fn name(&self) -> String {
        let (o, r) = match self {
            RepoResult::Finished(r) => (&r.owner, &r.name),
            RepoResult::Errored(r) => (&r.owner, &r.name),
        };

        format!("{o}/{r}")
    }
}

#[derive(Debug)]
pub struct RunMergeResults {
    pub results: Vec<RepoResult>,
    pub summary: RunSummary,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

pub trait RepoCheckState: private::Sealed {}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct RepoCheckInProgress(Vec<MergeResult>);
impl private::Sealed for RepoCheckInProgress {}
impl RepoCheckState for RepoCheckInProgress {}

#[derive(Debug)]
pub struct RepoCheckErrored(pub anyhow::Error);
impl private::Sealed for RepoCheckErrored {}
impl RepoCheckState for RepoCheckErrored {}
impl RepoCheckErrored {
    pub fn reason(&self) -> &anyhow::Error {
        &self.0
    }
}

#[cfg(test)]
impl serde::Serialize for RepoCheckErrored {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct RepoCheckFinished(pub Vec<MergeResult>);
impl private::Sealed for RepoCheckFinished {}
impl RepoCheckState for RepoCheckFinished {}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct RepoCheck<S: RepoCheckState> {
    pub owner: String,
    pub name: String,
    pub state: S,
}

impl RepoCheck<RepoCheckInProgress> {
    pub fn new(owner: &str, name: &str) -> Self {
        Self {
            owner: owner.to_string(),
            name: name.to_string(),
            state: RepoCheckInProgress(vec![]),
        }
    }

    pub fn add_merge_result(&mut self, result: MergeResult) {
        self.state.0.push(result);
    }

    pub fn record_error(self, error: anyhow::Error) -> RepoCheck<RepoCheckErrored> {
        RepoCheck {
            owner: self.owner,
            name: self.name,
            state: RepoCheckErrored(error),
        }
    }

    pub fn finish(self) -> RepoCheck<RepoCheckFinished> {
        RepoCheck {
            owner: self.owner,
            name: self.name,
            state: RepoCheckFinished(self.state.0),
        }
    }
}

impl RepoCheck<RepoCheckFinished> {
    pub fn results(&self) -> &Vec<MergeResult> {
        &self.state.0
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct PRCheck<S: PRCheckState> {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub pr_created_at: Option<DateTime<Utc>>,
    pub pr_updated_at: Option<DateTime<Utc>>,
    pub qualifications: Vec<Qualification>,
    pub state: S,
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum MergeResult {
    Qualified(PRCheck<PRCheckFinished>),
    Disqualified(PRCheck<PRDisqualified>),
    Errored(PRCheck<PRCheckErrored>),
}

impl MergeResult {
    pub fn no_failure(&self) -> bool {
        matches!(self, MergeResult::Qualified(_))
    }

    pub fn pr_number(&self) -> u64 {
        match self {
            MergeResult::Qualified(r) => r.number,
            MergeResult::Disqualified(r) => r.number,
            MergeResult::Errored(r) => r.number,
        }
    }

    pub fn pr_title(&self) -> &str {
        match self {
            MergeResult::Qualified(r) => &r.title,
            MergeResult::Disqualified(r) => &r.title,
            MergeResult::Errored(r) => &r.title,
        }
    }

    pub fn pr_url(&self) -> &str {
        match self {
            MergeResult::Qualified(r) => &r.url,
            MergeResult::Disqualified(r) => &r.url,
            MergeResult::Errored(r) => &r.url,
        }
    }

    pub fn pr_created_at(&self) -> Option<DateTime<Utc>> {
        match self {
            MergeResult::Qualified(r) => r.pr_created_at,
            MergeResult::Disqualified(r) => r.pr_created_at,
            MergeResult::Errored(r) => r.pr_created_at,
        }
    }

    pub fn pr_updated_at(&self) -> Option<DateTime<Utc>> {
        match self {
            MergeResult::Qualified(r) => r.pr_updated_at,
            MergeResult::Disqualified(r) => r.pr_updated_at,
            MergeResult::Errored(r) => r.pr_updated_at,
        }
    }

    pub fn qualifications(&self) -> &Vec<Qualification> {
        match self {
            MergeResult::Qualified(r) => &r.qualifications,
            MergeResult::Disqualified(r) => &r.qualifications,
            MergeResult::Errored(r) => &r.qualifications,
        }
    }
}

pub trait PRCheckState: private::Sealed {}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct PRCheckInProgress;
impl private::Sealed for PRCheckInProgress {}
impl PRCheckState for PRCheckInProgress {}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct PRDisqualified(pub Disqualification);
impl private::Sealed for PRDisqualified {}
impl PRCheckState for PRDisqualified {}

impl PRDisqualified {
    pub fn reason(&self) -> &Disqualification {
        &self.0
    }
}

#[derive(Debug)]
pub struct PRCheckErrored(pub anyhow::Error);
impl private::Sealed for PRCheckErrored {}
impl PRCheckState for PRCheckErrored {}
impl PRCheckErrored {
    pub fn reason(&self) -> &anyhow::Error {
        &self.0
    }
}

#[cfg(test)]
impl serde::Serialize for PRCheckErrored {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct PRCheckFinished;
impl private::Sealed for PRCheckFinished {}
impl PRCheckState for PRCheckFinished {}

impl From<&PullRequest> for PRCheck<PRCheckInProgress> {
    fn from(pr: &PullRequest) -> Self {
        Self {
            number: pr.number,
            title: pr.title.clone().unwrap_or_default(),
            url: pr
                .html_url
                .as_ref()
                .map(|url| url.to_string())
                .unwrap_or_default(),
            pr_created_at: pr.created_at,
            pr_updated_at: pr.updated_at,
            qualifications: vec![],
            state: PRCheckInProgress,
        }
    }
}

impl PRCheck<PRCheckInProgress> {
    pub fn add_qualification(&mut self, q: Qualification) {
        self.qualifications.push(q);
    }

    pub fn disqualify(self, dq: Disqualification) -> PRCheck<PRDisqualified> {
        PRCheck {
            number: self.number,
            title: self.title,
            url: self.url,
            pr_created_at: self.pr_created_at,
            pr_updated_at: self.pr_updated_at,
            qualifications: self.qualifications,
            state: PRDisqualified(dq),
        }
    }

    pub fn record_error(self, error: anyhow::Error) -> PRCheck<PRCheckErrored> {
        PRCheck {
            number: self.number,
            title: self.title,
            url: self.url,
            pr_created_at: self.pr_created_at,
            pr_updated_at: self.pr_updated_at,
            qualifications: self.qualifications,
            state: PRCheckErrored(error),
        }
    }

    pub fn finish(self) -> PRCheck<PRCheckFinished> {
        PRCheck {
            number: self.number,
            title: self.title,
            url: self.url,
            pr_created_at: self.pr_created_at,
            pr_updated_at: self.pr_updated_at,
            qualifications: self.qualifications,
            state: PRCheckFinished,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct MergedPR {
    pub repo: String,
    pub title: String,
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct RunDisqualification {
    pub pr_url: String,
    pub reason: String,
}

#[derive(Debug, Default)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct RunSummary {
    pub disqualifications: Vec<RunDisqualification>,
    pub num_errors: u16,
    pub prs_merged: Vec<MergedPR>,
}

impl RunSummary {
    pub fn from_results(results: &[RepoResult], did_execute: bool) -> Self {
        let mut num_errors = 0;
        let mut disqualifications = vec![];
        let mut prs_merged = vec![];

        for result in results {
            match result {
                RepoResult::Errored(_) => {
                    num_errors += 1;
                }
                RepoResult::Finished(repo_check) => {
                    for merge_result in repo_check.results() {
                        match merge_result {
                            MergeResult::Qualified(pr_check) => {
                                if did_execute {
                                    prs_merged.push(MergedPR {
                                        repo: result.name(),
                                        title: pr_check.title.clone(),
                                    });
                                }
                            }
                            MergeResult::Disqualified(pr_check) => {
                                disqualifications.push(RunDisqualification {
                                    pr_url: pr_check.url.clone(),
                                    reason: pr_check.state.reason().summary(),
                                });
                            }
                            MergeResult::Errored(_) => {
                                num_errors += 1;
                            }
                        }
                    }
                }
            }
        }

        Self {
            disqualifications,
            num_errors,
            prs_merged,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum Qualification {
    Head(String),
    Author(String),
    Check { name: String, conclusion: String },
    State(String),
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum Disqualification {
    Head(String),
    Author(Option<String>),
    Check {
        name: String,
        conclusion: Option<String>,
    },
    State(Option<String>),
}

impl Disqualification {
    pub fn summary(&self) -> String {
        match self {
            Disqualification::Head(_) => "head didn't match".to_string(),
            Disqualification::Author(author) => match author {
                Some(a) => format!("author untrusted: {a}"),
                None => "author unknown".to_string(),
            },
            Disqualification::Check { name, conclusion } => match conclusion {
                Some(c) => format!("check {name}: {c}"),
                None => format!("check {name}: unknown conclusion"),
            },
            Disqualification::State(state) => match state {
                Some(s) => format!("state: {s}"),
                None => "state: unknown".to_string(),
            },
        }
    }
}

pub struct ReportConfig {
    pub output_path: PathBuf,
    pub custom_template: Option<String>,
    pub title: String,
    pub num_runs: u8,
    pub open_report: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_yaml_snapshot;

    const OWNER: &str = "dhth";
    const REPO: &str = "mrj";
    const PR_TITLE: &str = "build: bump clap from 4.5.39 to 4.5.40";
    const PR_URL: &str = "https://github.com/dhth/mrj/pull/1";
    const PR_HEAD: &str = "dependabot/cargo/clap-4.5.40";

    #[test]
    fn run_summary_works_as_expected() {
        // GIVEN
        let repo_result = RepoResult::Finished(RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![
                merge_result_disqualified_unmatched_head(),
                merge_result_disqualified_unknown_author(),
                merge_result_disqualified_untrusted_author(),
                merge_result_disqualified_check_with_unknown_conclusion(),
                merge_result_disqualified_failed_check(),
                merge_result_disqualified_unknown_state(),
                merge_result_disqualified_dirty_state(),
                merge_result_errored(),
                merge_result_qualified(),
            ]),
        });

        // WHEN
        let summary = RunSummary::from_results(&[repo_result], true);

        // THEN
        assert_yaml_snapshot!(summary, @r#"
        disqualifications:
          - pr_url: "https://github.com/dhth/mrj/pull/1"
            reason: "head didn't match"
          - pr_url: "https://github.com/dhth/mrj/pull/1"
            reason: author unknown
          - pr_url: "https://github.com/dhth/mrj/pull/1"
            reason: "author untrusted: untrusted-author"
          - pr_url: "https://github.com/dhth/mrj/pull/1"
            reason: "check lint: unknown conclusion"
          - pr_url: "https://github.com/dhth/mrj/pull/1"
            reason: "check lint: failure"
          - pr_url: "https://github.com/dhth/mrj/pull/1"
            reason: "state: unknown"
          - pr_url: "https://github.com/dhth/mrj/pull/1"
            reason: "state: dirty"
        num_errors: 1
        prs_merged:
          - repo: dhth/mrj
            title: "build: bump clap from 4.5.39 to 4.5.40"
        "#);
    }

    fn merge_result_disqualified_unmatched_head() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![],
            state: PRDisqualified(Disqualification::Head("improve-tests".to_string())),
        })
    }

    fn merge_result_disqualified_unknown_author() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::Author(None)),
        })
    }

    fn merge_result_disqualified_untrusted_author() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
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

    fn merge_result_disqualified_check_with_unknown_conclusion() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::Check {
                name: "lint".to_string(),
                conclusion: None,
            }),
        })
    }

    fn merge_result_disqualified_failed_check() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::Check {
                name: "lint".to_string(),
                conclusion: Some("failure".to_string()),
            }),
        })
    }

    fn merge_result_disqualified_unknown_state() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::State(None)),
        })
    }

    fn merge_result_disqualified_dirty_state() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::State(Some("dirty".to_string()))),
        })
    }

    fn merge_result_errored() -> MergeResult {
        MergeResult::Errored(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRCheckErrored(anyhow::anyhow!("couldn't merge PR: GitHub API was down")),
        })
    }

    fn merge_result_qualified() -> MergeResult {
        MergeResult::Qualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: None,
            pr_updated_at: None,
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRCheckFinished,
        })
    }
}

mod private {
    pub trait Sealed {}
}
