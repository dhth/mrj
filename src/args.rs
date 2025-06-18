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
        #[arg(long = "output-to-file", short = 'o')]
        output_to_file: bool,
        /// Whether to write mrj's log of events to a file
        #[arg(
            long = "output-path",
            value_name = "FILE",
            default_value = "output.txt",
            value_parser = validate_txt_path,
        )]
        output_path: PathBuf,
        /// Whether to write merge summary to a file
        #[arg(long = "summary", short = 's')]
        summary: bool,
        /// File to write summary to
        #[arg(long = "summary-path",
            value_name = "FILE",
            default_value = "summary.txt",
            value_parser = validate_txt_path,
            )]
        summary_path: PathBuf,
        /// Whether to show disqualifications in the summary
        #[arg(long = "summarize-disqualifications", short = 'd')]
        summarize_disqualifications: bool,
        /// Whether to show information for repos with no PRs
        #[arg(long = "show-repos-with-no-prs", short = 'n')]
        show_repos_with_no_prs: bool,
        /// Whether to show information for PRs from untrusted authors
        #[arg(long = "show-prs-from-untrusted-authors", short = 'u')]
        show_prs_from_untrusted_authors: bool,
        /// Whether to show information for PRs where head doesn't match configured pattern
        #[arg(long = "show-unmatched-head-prs", short = 'H')]
        show_prs_with_unmatched_head: bool,
        /// Whether to actually merge PRs; mrj operates in "dry-run mode" by default
        #[arg(long = "execute", short = 'e')]
        execute: bool,
        /// Whether to use output text to stdout without color
        #[arg(long = "plain", short = 'p')]
        plain_stdout: bool,
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
        /// Maximum number of runs to keep in the report (allowed range: [1, 100])
        #[arg(
            long = "num-runs",
            short = 'n',
            value_name="NUMBER",
            default_value_t=10,
            value_parser = clap::value_parser!(u8).range(1..=100),
            )]
        num_runs: u8,
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
                output_to_file: output,
                output_path,
                summary,
                summary_path,
                summarize_disqualifications,
                show_repos_with_no_prs,
                show_prs_from_untrusted_authors,
                show_prs_with_unmatched_head,
                execute,
                plain_stdout,
            } => format!(
                r#"
command                           : Run
config file                       : {}
repos (overridden)                : {:?}
output to file                    : {}
output file                       : {}
write summary                     : {}
summary file                      : {}
summarize disqualifications       : {}
show repos with no prs            : {}
show prs from untrusted authors   : {}
show prs with unmatched head      : {}
execute                           : {}
plain stdout                      : {}
"#,
                config_file.to_string_lossy(),
                repos.iter().map(|r| r.to_string()).collect::<Vec<String>>(),
                output,
                output_path.to_string_lossy(),
                summary,
                summary_path.to_string_lossy(),
                summarize_disqualifications,
                show_repos_with_no_prs,
                show_prs_from_untrusted_authors,
                show_prs_with_unmatched_head,
                execute,
                plain_stdout,
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
                    num_runs,
                } => format!(
                    r#"
command                  : Generate report
output file              : {}
open report              : {}
num runs                 : {}
"#,
                    output_path.to_string_lossy(),
                    open_report,
                    num_runs,
                ),
            },
        };

        f.write_str(&output)
    }
}

fn validate_txt_path(s: &str) -> Result<PathBuf, String> {
    if s.ends_with(".txt") {
        Ok(PathBuf::from(s))
    } else {
        Err(String::from("file must have a .txt extension"))
    }
}
