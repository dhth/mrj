use crate::config::Config;
use anyhow::Context;
use colored::Colorize;
use octocrab::Octocrab;
use octocrab::{
    models::pulls::MergeableState,
    params::{State, pulls::MergeMethod, repos::Commitish},
};

const BANNER: &str = include_str!("assets/banner.txt");

pub async fn merge_pr(
    client: Octocrab,
    owner: &str,
    repo: &str,
    config: Config,
    dry_run: bool,
) -> anyhow::Result<()> {
    print_banner(dry_run);

    let page = client
        .pulls(owner, repo)
        .list()
        .state(State::Open)
        .per_page(100)
        .send()
        .await
        .context("couldn't get PRs")?;

    for pull_request in &page {
        print_intro(&format!(
            "-> checking PR #{}: {}",
            pull_request.number,
            pull_request.title.clone().unwrap_or_default()
        ));

        match &pull_request.user {
            Some(trusted_user) if config.trusted_authors.contains(&trusted_user.login) => {
                print_info(&format!(
                    "author \"{}\" is in the list of trusted users",
                    trusted_user.login
                ));
            }
            Some(other_user) => {
                print_warning(&format!(
                    "author \"{}\" is not in the list of trusted users",
                    other_user.login
                ));
                continue;
            }
            None => {
                print_warning(
                    "Github sent an empty user; skipping as I can't make any assumptions here",
                );
                continue;
            }
        }

        let pr = client
            .pulls(owner, repo)
            .get(pull_request.number)
            .await
            .context("couldn't get details")?;

        let pr_head_ref = pr.head.sha;

        let checks = client
            .checks(owner, repo)
            .list_check_runs_for_git_ref(Commitish::from(pr_head_ref.clone()))
            .send()
            .await
            .context("couldn't get pr checks")?;

        let mut skip = false;
        for check in &checks.check_runs {
            match check.conclusion.as_deref() {
                Some("success") => {
                    print_info(&format!("check \"{}\": ✅", check.name));
                }
                Some("skipped") => {
                    if config.merge_if_checks_skipped.unwrap_or(false) {
                        print_info(&format!("check \"{}\": skipped", check.name));
                    } else {
                        print_warning(&format!(
                            "check \"{}\" skipped; merge_if_checks_skipped is false, so skipping",
                            check.name
                        ));
                        skip = true;
                        break;
                    }
                }
                Some(non_successful_conclusion) => {
                    print_warning(&format!(
                        "check \"{}\" {} ❌; skipping",
                        check.name, non_successful_conclusion
                    ));
                    skip = true;
                    break;
                }
                None => {
                    print_warning(&format!(
                        "Github returned with an empty conclusion for the check {}; skipping as I can't make any assumptions here",
                        check.name,
                    ));
                    skip = true;
                    break;
                }
            }
        }

        if skip {
            continue;
        }

        if !&checks.check_runs.is_empty() {
            print_info("checks look good");
        }

        match pr.mergeable_state.as_ref() {
            Some(state) => match state {
                MergeableState::Clean => {
                    print_info("state: \"clean\" ✅");
                }
                MergeableState::Blocked => {
                    if config.merge_if_blocked.unwrap_or(false) {
                        print_info("state: \"blocked\" ✅ (merge_if_blocked is true)");
                    } else {
                        print_warning("PR state is blocked, skipping");
                        continue;
                    }
                }
                other => {
                    print_warning(&format!(
                        "PR state is not clean, skipping; state: {:?}",
                        other
                    ));
                    continue;
                }
            },
            None => {
                print_warning(
                    "Github returned with an empty mergeable state; skipping as I can't make any assumptions here",
                );
                continue;
            }
        }

        print_info("PR matches all criteria, merging...");

        if !dry_run {
            client
                .pulls(owner, repo)
                .merge(pr.number)
                .method(config.merge_type.merge_method())
                .send()
                .await
                .context("couldn't merge PR")?;
            print_success("PR merged! 🎉");

            break;
        }
    }

    Ok(())
}

fn print_banner(dry_run: bool) {
    println!("{}", BANNER.green().bold());

    if dry_run {
        println!("{}", "                         dry run".yellow());
    }
}

fn print_intro(msg: &str) {
    println!("\n{}", msg.purple());
}

fn print_info(msg: &str) {
    println!("\t{}", msg.blue());
}

fn print_warning(msg: &str) {
    println!("\t{}", msg.yellow());
}

fn print_success(msg: &str) {
    println!("\t{}", msg.green());
}
