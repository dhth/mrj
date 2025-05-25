mod args;
mod config;
mod domain;
mod merge;

use anyhow::Context;
use args::Args;
use args::{ConfigCommand, MrjCommand, ReportCommand};
use clap::Parser;
use config::get_config;
use merge::merge_prs;
use std::env::VarError;

const TOKEN_ENV_VAR: &str = "MRJ_TOKEN";
const SAMPLE_CONFIG: &str = include_str!("./assets/sample-config.toml");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.debug {
        print!("DEBUG INFO\n{}", args);
        return Ok(());
    }

    match args.command {
        MrjCommand::Run {
            config_file,
            repos,
            output,
            output_path,
            dry_run,
        } => {
            let config = get_config(config_file)?;

            if config.repos.is_empty() && repos.is_empty() {
                anyhow::bail!("no repos to run for");
            }

            let token = std::env::var(TOKEN_ENV_VAR).map_err(|err| match err {
                VarError::NotPresent => anyhow::anyhow!("{} is not set", TOKEN_ENV_VAR),
                VarError::NotUnicode(_) => {
                    anyhow::anyhow!("{} is not valid unicode", TOKEN_ENV_VAR)
                }
            })?;

            let client = octocrab::instance()
                .user_access_token(token)
                .context("couldn't authorize github client")?;

            merge_prs(client, config, repos, output, output_path, dry_run).await?;
        }
        MrjCommand::Config { config_command } => match config_command {
            ConfigCommand::Validate { config_file } => {
                get_config(config_file)?;
                println!("config looks good ✅");
            }
            ConfigCommand::Sample => print!("{}", SAMPLE_CONFIG),
        },
        MrjCommand::Report { report_command } => match report_command {
            ReportCommand::Generate => println!("generating!"),
        },
    }

    Ok(())
}
