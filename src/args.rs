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
        /// Whether to write output to a file
        #[arg(long = "output", short = 'o')]
        output: bool,
        /// Whether to write mrj's log of events to a file
        #[arg(
            long = "output-path",
            value_name = "FILE",
            default_value = "output.txt",
            value_parser = validate_output_path,
        )]
        output_path: PathBuf,
        /// Whether to write merge stats to a file
        #[arg(long = "stats", short = 's')]
        stats: bool,
        /// File to write stats to
        #[arg(long = "stats-path",
            value_name = "FILE",
            default_value = "stats.csv",
            value_parser = validate_stats_path,
            )]
        stats_path: PathBuf,
        /// Whether to only print out information without merging any PRs
        #[arg(long = "dry-run", short = 'd')]
        dry_run: bool,
    },
    /// Interact with mrj's config
    Config {
        #[command(subcommand)]
        config_command: ConfigCommand,
    },
    /// Generate report from mrj runs
    Report {
        #[command(subcommand)]
        report_command: ReportCommand,
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

#[derive(Subcommand, Debug)]
pub enum ReportCommand {
    /// Generate a report
    Generate {
        /// File containing the output of "mrj run"
        #[arg(
            long = "output-path",
            short = 'p',
            value_name = "PATH",
            default_value = "output.txt"
        )]
        output_path: PathBuf,
        /// Whether to open report in the browser
        #[arg(long = "open", short = 'o')]
        open_report: bool,
    },
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
                output,
                output_path,
                stats,
                stats_path,
                dry_run,
            } => format!(
                r#"
command                  : Run
config file              : {}
repos (overridden)       : {:?}
write output             : {}
output file              : {}
write stats              : {}
stats file               : {}
dry run                  : {}
"#,
                config_file.to_string_lossy(),
                repos.iter().map(|r| r.to_string()).collect::<Vec<String>>(),
                output,
                output_path.to_string_lossy(),
                stats,
                stats_path.to_string_lossy(),
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
            MrjCommand::Report { report_command } => match report_command {
                ReportCommand::Generate {
                    output_path,
                    open_report,
                } => format!(
                    r#"
command                  : Generate report
output file              : {}
open report              : {}
"#,
                    output_path.to_string_lossy(),
                    open_report,
                ),
            },
        };

        f.write_str(&output)
    }
}

fn validate_output_path(s: &str) -> Result<PathBuf, String> {
    if s.ends_with(".txt") {
        Ok(PathBuf::from(s))
    } else {
        Err(String::from("output file must have a .txt extension"))
    }
}

fn validate_stats_path(s: &str) -> Result<PathBuf, String> {
    if s.ends_with(".csv") {
        Ok(PathBuf::from(s))
    } else {
        Err(String::from("stats file must have a .csv extension"))
    }
}
