use octocrab::params::pulls::MergeMethod;
use regex::Regex;
use serde::{
    Deserialize, Deserializer, Serialize,
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

#[derive(Debug)]
pub struct RepoResult {
    pub owner: String,
    pub name: String,
    pub result: anyhow::Result<Vec<PRResult>>,
}

impl RepoResult {
    pub fn new(owner: &str, name: &str) -> Self {
        Self {
            owner: owner.to_string(),
            name: name.to_string(),
            result: Ok(vec![]),
        }
    }

    pub fn add_pr_result(&mut self, result: PRResult) {
        if let Ok(pr_results) = &mut self.result {
            pr_results.push(result);
        }
    }

    pub fn record_error(mut self, error: anyhow::Error) -> Self {
        self.result = Err(error);
        self
    }

    pub fn name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

#[derive(Debug)]
pub struct PRResult {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub qualifications: Vec<Qualification>,
    pub failure: Option<Failure>,
}

impl PRResult {
    pub fn new(number: u64, title: &str, url: &str) -> Self {
        Self {
            number,
            title: title.to_string(),
            url: url.to_string(),
            qualifications: vec![],
            failure: None,
        }
    }

    pub fn add_qualification(&mut self, q: Qualification) {
        self.qualifications.push(q);
    }

    pub fn disqualify(mut self, dq: Disqualification) -> Self {
        self.failure = Some(Failure::Disqualification(dq));
        self
    }

    pub fn record_error(mut self, error: anyhow::Error) -> Self {
        self.failure = Some(Failure::Error(error));
        self
    }

    pub fn merged(&self) -> bool {
        self.failure.is_none()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RunStats {
    pub num_merges: u16,
    pub num_disqualifications: u16,
    pub num_errors: u16,
}

impl RunStats {
    pub fn record_merge(&mut self) {
        self.num_merges += 1;
    }

    pub fn record_disqualification(&mut self) {
        self.num_disqualifications += 1;
    }

    pub fn record_error(&mut self) {
        self.num_errors += 1;
    }
}

#[derive(Debug, Clone)]
pub enum Qualification {
    Head(String),
    User(String),
    Check { name: String, conclusion: String },
    State(String),
}

#[derive(Debug)]
pub enum Failure {
    Disqualification(Disqualification),
    Error(anyhow::Error),
}

#[derive(Debug)]
pub enum Disqualification {
    Head(String),
    User(Option<String>),
    Check {
        name: String,
        conclusion: Option<String>,
    },
    State(Option<String>),
}
