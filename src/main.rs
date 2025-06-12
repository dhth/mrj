mod args;
mod auth;
mod config;
mod domain;
mod merge;
mod report;

use anyhow::Context;
use args::Args;
use args::{ConfigCommand, MrjCommand, ReportCommand};
use auth::get_token;
use clap::Parser;
use config::get_config;
use merge::{RunBehaviours, merge_prs};
use report::generate_report;

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
            summary,
            summary_path,
            show_repos_with_no_prs,
            show_prs_from_untrusted_authors,
            execute,
        } => {
            let config = get_config(config_file)?;

            if config.repos.is_empty() && repos.is_empty() {
                anyhow::bail!("no repos to run for");
            }

            let token = get_token()?;

            octocrab::initialise(
                octocrab::Octocrab::builder()
                    .user_access_token(token)
                    .build()
                    .context("couldn't build github client")?,
            );
            let client = octocrab::instance();

            let run_behaviours = RunBehaviours {
                output,
                output_path: &output_path,
                summary,
                summary_path: &summary_path,
                show_repos_with_no_prs,
                show_prs_from_untrusted_authors,
                execute,
            };
            merge_prs(client, config, repos, run_behaviours).await?;
        }
        MrjCommand::Config { config_command } => match config_command {
            ConfigCommand::Validate { config_file } => {
                get_config(config_file)?;
                println!("config looks good ✅");
            }
            ConfigCommand::Sample => print!("{}", SAMPLE_CONFIG),
        },
        MrjCommand::Report { report_command } => match report_command {
            ReportCommand::Generate {
                output_path,
                open_report,
                num_runs,
            } => generate_report(&output_path, open_report, num_runs)?,
        },
    }

    Ok(())
}
