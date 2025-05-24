use crate::domain::Repo;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// mrj merges your open PRs
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: MrjCommand,
    /// Output debug information without doing anything
    #[arg(long = "debug", global = true)]
    pub debug: bool,
}

#[derive(Subcommand, Debug)]
pub enum MrjCommand {
    /// Check for open PRs and merge them
    #[command(name = "run")]
    Run {
        /// Path to mrj's config file
        #[arg(
            long = "config",
            short = 'c',
            value_name = "PATH",
            default_value = "mrj.toml"
        )]
        config_file: PathBuf,
        /// Repos to run for (will override repos in config)
        #[arg(long = "repos",
            short = 'r',
            value_name = "STRING,STRING",
            value_delimiter = ',',
            value_parser = validate_repo
            )]
        repos: Vec<Repo>,
        /// Whether to only print out information without merging any PRs
        #[arg(long = "dry-run", short = 'd')]
        dry_run: bool,
    },
    /// Interact with mrj's config
    Config {
        #[command(subcommand)]
        config_command: ConfigCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Validate mrj's config
    Validate {
        /// Path to mrj's config file
        #[arg(
            long = "path",
            short = 'p',
            value_name = "PATH",
            default_value = "mrj.toml"
        )]
        config_file: PathBuf,
    },
    /// Print out a sample config
    Sample,
}

fn validate_repo(value: &str) -> Result<Repo, String> {
    Repo::try_from(value)
}

impl std::fmt::Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match &self.command {
            MrjCommand::Run {
                config_file,
                repos,
                dry_run,
            } => format!(
                r#"
command                  : Run
config file              : {}
repos (overridden)       : {:?}
dry run                  : {}
"#,
                config_file.to_string_lossy(),
                repos.iter().map(|r| r.to_string()).collect::<Vec<String>>(),
                dry_run
            ),
            MrjCommand::Config { config_command } => match config_command {
                ConfigCommand::Validate { config_file } => format!(
                    r#"
command                  : Validate config
config file              : {}
"#,
                    config_file.to_string_lossy(),
                ),

                ConfigCommand::Sample => r#"
command                  : Show sample config
"#
                .to_string(),
            },
        };

        f.write_str(&output)
    }
}
