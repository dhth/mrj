use chrono::{DateTime, Utc};
use octocrab::models::pulls::PullRequest;
use octocrab::params::Direction;
use octocrab::params::pulls::{MergeMethod, Sort};
use regex::Regex;
use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
};
use std::fmt::{self, Display};

#[derive(Debug)]
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

#[derive(Debug, serde::Deserialize, PartialEq)]
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

#[derive(Debug, serde::Deserialize, PartialEq)]
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

        format!("{}/{}", o, r)
    }
}

pub trait RepoCheckState: private::Sealed {}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct RepoCheckFinished(pub Vec<MergeResult>);
impl private::Sealed for RepoCheckFinished {}
impl RepoCheckState for RepoCheckFinished {}

#[derive(Debug)]
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
pub struct PRCheckInProgress;
impl private::Sealed for PRCheckInProgress {}
impl PRCheckState for PRCheckInProgress {}

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug, Default)]
pub struct RunSummary {
    pub num_repos: usize,
    pub num_repos_with_no_prs: usize,
    pub disqualifications: Vec<(String, String)>,
    pub num_errors: u16,
    pub prs_merged: Vec<MergedPR>,
}

impl RunSummary {
    pub fn record_repo(&mut self) {
        self.num_repos += 1;
    }

    pub fn record_repo_with_no_prs(&mut self) {
        self.num_repos_with_no_prs += 1;
    }

    pub fn record_disqualification(&mut self, pr_url: &str, disqualification: &Disqualification) {
        let disqualification_summary = match disqualification {
            Disqualification::Head(_) => "head didn't match".to_string(),
            Disqualification::Author(author) => match author {
                Some(a) => format!("author {} untrusted", a),
                None => "author unknown".to_string(),
            },
            Disqualification::Check { name, conclusion } => match conclusion {
                Some(c) => format!("check {}: {}", name, c),
                None => format!("check {}: unknown conclusion", name),
            },
            Disqualification::State(state) => match state {
                Some(s) => format!("state: {}", s),
                None => "state: unknown".to_string(),
            },
        };

        self.disqualifications
            .push((pr_url.to_string(), disqualification_summary));
    }

    pub fn record_error(&mut self) {
        self.num_errors += 1;
    }

    pub fn record_merged_pr(&mut self, pr: MergedPR) {
        self.prs_merged.push(pr);
    }
}

#[derive(Debug)]
pub struct MergedPR {
    pub repo: String,
    pub title: String,
}

#[derive(Debug)]
pub enum Qualification {
    Head(String),
    Author(String),
    Check { name: String, conclusion: String },
    State(String),
}

#[derive(Debug)]
pub enum Disqualification {
    Head(String),
    Author(Option<String>),
    Check {
        name: String,
        conclusion: Option<String>,
    },
    State(Option<String>),
}

mod private {
    pub trait Sealed {}
}
