#![allow(unused)]

mod args;
mod config;
mod merge;

use anyhow::Context;
use args::Args;
use clap::Parser;
use config::Config;
use merge::merge_pr;
use std::env::VarError;

const TOKEN_ENV_VAR: &str = "MRJ_TOKEN";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config_bytes =
        std::fs::read_to_string(args.config_file).context("couldn't read config file")?;
    let config: Config = toml::from_str(&config_bytes)?;

    let token = std::env::var(TOKEN_ENV_VAR).map_err(|err| match err {
        VarError::NotPresent => anyhow::anyhow!("{} is not set", TOKEN_ENV_VAR),
        VarError::NotUnicode(_) => anyhow::anyhow!("{} is not valid unicode", TOKEN_ENV_VAR),
    })?;

    let client = octocrab::instance()
        .user_access_token(token)
        .context("couldn't authorize github client")?;

    merge_pr(client, config, args.dry_run).await?;

    Ok(())
}
