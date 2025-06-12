use crate::domain::{HeadPattern, MergeType, Repo, SortBy, SortDirection};
use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub repos: Vec<Repo>,
    pub trusted_authors: Vec<String>,
    pub base_branch: Option<String>,
    pub head_pattern: Option<HeadPattern>,
    #[serde(default = "default_false")]
    pub merge_if_blocked: bool,
    #[serde(default = "default_true")]
    pub merge_if_checks_skipped: bool,
    pub merge_type: MergeType,
    #[serde(default = "default_sort")]
    pub sort_by: SortBy,
    #[serde(default = "default_sort_direction")]
    pub sort_direction: SortDirection,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_sort() -> SortBy {
    SortBy::Created
}

fn default_sort_direction() -> SortDirection {
    SortDirection::Ascending
}

pub fn get_config(config_path: PathBuf) -> anyhow::Result<Config> {
    let config_str = std::fs::read_to_string(&config_path).with_context(|| {
        format!(
            "couldn't read config file \"{}\"",
            &config_path.to_string_lossy()
        )
    })?;
    let config: Config = parse_config(&config_str)?;

    Ok(config)
}

fn parse_config(config_str: &str) -> anyhow::Result<Config> {
    let config: Config = toml::from_str(config_str)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    //-------------//
    //  SUCCESSES  //
    //-------------//

    #[test]
    fn parsing_correct_config_with_all_props_works() {
        // GIVEN
        let config_str = r#"
repos = [
    "user/repo-1",
    "user/repo-2",
    "user/repo-3",
]
trusted_authors = ["dependabot[bot]"]
base_branch = "main"
head_pattern = "(dependabot|update)"
merge_if_blocked = true
merge_if_checks_skipped = true
merge_type = "squash"
sort_by = "updated"
sort_direction = "desc"
"#;

        // WHEN
        let config = parse_config(config_str).expect("config should've been parsed");

        // THEN
        assert_eq!(config.repos.len(), 3);
        assert_eq!(config.sort_by, SortBy::Updated);
        assert_eq!(config.sort_direction, SortDirection::Descending);
    }

    #[test]
    fn parsing_correct_config_with_all_mandatory_props_only() {
        // GIVEN
        let config_str = r#"
repos = [
    "user/repo-1",
    "user/repo-2",
    "user/repo-3",
]
trusted_authors = ["dependabot[bot]"]
merge_type = "squash"
"#;

        // WHEN
        let config = parse_config(config_str).expect("config should've been parsed");

        // THEN
        assert_eq!(config.repos.len(), 3);
        assert_eq!(config.trusted_authors.len(), 1);
        assert_eq!(config.merge_if_blocked, false);
        assert_eq!(config.merge_if_checks_skipped, true);
        assert_eq!(config.sort_by, SortBy::Created);
        assert_eq!(config.sort_direction, SortDirection::Ascending);
    }

    //-------------//
    //  FAILURES   //
    //-------------//

    #[test]
    fn parsing_invalid_toml_fails() {
        // GIVEN
        let config_str = r#"
repos = 
    "user/repo-1",
    "user/repo-2",
    "user/repo-3",

trusted_authors = ["dependabot[bot]"]
merge_type = "squash"
"#;

        // WHEN
        // THEN
        let _ = parse_config(config_str).expect_err("config shouldn't have been parsed");
    }

    #[test]
    fn parsing_invalid_regex_for_head_pattern_fails() {
        // GIVEN
        let config_str = r#"
repos = [
    "user/repo-1",
    "user/repo-2",
    "user/repo-3",
]

trusted_authors = ["dependabot[bot]"]
head_pattern = "abc)"
merge_type = "squash"
"#;

        // WHEN
        // THEN
        let _ = parse_config(config_str).expect_err("config shouldn't have been parsed");
    }

    #[test]
    fn parsing_invalid_merge_type_fails() {
        // GIVEN
        let config_str = r#"
repos = [
    "user/repo-1",
    "user/repo-2",
    "user/repo-3",
]

trusted_authors = ["dependabot[bot]"]
merge_type = "unknown"
"#;

        // WHEN
        // THEN
        let _ = parse_config(config_str).expect_err("config shouldn't have been parsed");
    }

    #[test]
    fn parsing_invalid_sort_by_fails() {
        // GIVEN
        let config_str = r#"
repos = [
    "user/repo-1",
    "user/repo-2",
    "user/repo-3",
]

trusted_authors = ["dependabot[bot]"]
merge_type = "squash"
sort_by = "unknown"
"#;

        // WHEN
        // THEN
        let _ = parse_config(config_str).expect_err("config shouldn't have been parsed");
    }

    #[test]
    fn parsing_invalid_sort_direction_fails() {
        // GIVEN
        let config_str = r#"
repos = [
    "user/repo-1",
    "user/repo-2",
    "user/repo-3",
]

trusted_authors = ["dependabot[bot]"]
merge_type = "squash"
sort_direction = "unknown"
"#;

        // WHEN
        // THEN
        let _ = parse_config(config_str).expect_err("config shouldn't have been parsed");
    }
}
