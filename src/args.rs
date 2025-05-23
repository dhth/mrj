use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// mrj merges your open PRs
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: MrjCommand,
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
