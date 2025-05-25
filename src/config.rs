use crate::domain::{HeadPattern, MergeType, Repo};
use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;

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

pub fn get_config(config_path: PathBuf) -> anyhow::Result<Config> {
    let config_bytes = std::fs::read_to_string(&config_path).with_context(|| {
        format!(
            "couldn't read config file \"{}\"",
            &config_path.to_string_lossy()
        )
    })?;
    let config: Config = toml::from_str(&config_bytes)?;

    Ok(config)
}
