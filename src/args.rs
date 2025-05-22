use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// mrj merges your open PRs
#[derive(Parser, Debug)]
pub struct Args {
    /// Path to mrj's config file
    #[arg(
        long = "config",
        short = 'c',
        value_name = "PATH",
        default_value = "mrj.toml"
    )]
    pub config_file: PathBuf,
    /// Whether to only print out information without merging any PRs
    #[arg(long = "dry-run", short = 'd')]
    pub dry_run: bool,
}
