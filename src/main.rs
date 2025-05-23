#![allow(unused)]

mod args;
mod config;
mod merge;

use anyhow::Context;
use args::Args;
use args::{ConfigCommand, MrjCommand};
use clap::Parser;
use config::{Config, get_config};
use merge::merge_pr;
use std::env::VarError;

const TOKEN_ENV_VAR: &str = "MRJ_TOKEN";
const SAMPLE_CONFIG: &str = include_str!("./assets/sample-config.toml");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        MrjCommand::Run {
            config_file,
            dry_run,
        } => {
            let config = get_config(config_file)?;

            let token = std::env::var(TOKEN_ENV_VAR).map_err(|err| match err {
                VarError::NotPresent => anyhow::anyhow!("{} is not set", TOKEN_ENV_VAR),
                VarError::NotUnicode(_) => {
                    anyhow::anyhow!("{} is not valid unicode", TOKEN_ENV_VAR)
                }
            })?;

            let client = octocrab::instance()
                .user_access_token(token)
                .context("couldn't authorize github client")?;

            merge_pr(client, config, dry_run).await?;
        }
        MrjCommand::Config { config_command } => match config_command {
            ConfigCommand::Validate { config_file } => {
                get_config(config_file)?;
                println!("config looks good ✅");
            }
            ConfigCommand::Sample => print!("{}", SAMPLE_CONFIG),
        },
    }

    Ok(())
}
