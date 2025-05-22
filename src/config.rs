use octocrab::params::pulls::MergeMethod;
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

#[derive(Debug, Deserialize)]
pub struct Config {
    pub trusted_authors: Vec<String>,
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
