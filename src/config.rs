use octocrab::params::pulls::MergeMethod;
use regex::Regex;
use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
};
use std::fmt;

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

#[derive(Debug)]
pub struct Repo {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug)]
pub struct HeadPattern {
    pub re: Regex,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub repos: Vec<Repo>,
    pub trusted_authors: Vec<String>,
    pub base_branch: Option<String>,
    pub head_pattern: Option<HeadPattern>,
    pub merge_if_blocked: Option<bool>,
    pub merge_if_checks_skipped: Option<bool>,
    pub merge_type: MergeType,
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
