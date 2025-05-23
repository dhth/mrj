use crate::config::Config;
use anyhow::Context;
use colored::Colorize;
use octocrab::Octocrab;
use octocrab::{
    models::pulls::MergeableState,
    params::{State, pulls::MergeMethod, repos::Commitish},
};

const BANNER: &str = include_str!("assets/banner.txt");
const AUTHOR: &str = "[ author ]  ";
const CHECK: &str = "[ check  ]  ";
const STATE: &str = "[ state  ]  ";

pub async fn merge_pr(client: Octocrab, config: Config, dry_run: bool) -> anyhow::Result<()> {
    print_banner(dry_run);

    for repo in &config.repos {
        let page = client
            .pulls(&repo.owner, &repo.repo)
            .list()
            .state(State::Open)
            .per_page(100)
            .send()
            .await
            .context("couldn't get PRs")?;
        print_repo_info(&repo.repo);

        if page.items.is_empty() {
            println!();
            print_warning("no PRs");
            continue;
        }

        for pull_request in &page {
            print_pr_info(&format!(
                "-> checking PR #{}: {}",
                pull_request.number,
                pull_request.title.clone().unwrap_or_default()
            ));

            match &pull_request.user {
                Some(trusted_user) if config.trusted_authors.contains(&trusted_user.login) => {
                    print_info(&format!(
                        "{} \"{}\" is in the list of trusted users",
                        AUTHOR, trusted_user.login
                    ));
                }
                Some(other_user) => {
                    print_warning(&format!(
                        "{} \"{}\" is not in the list of trusted users",
                        AUTHOR, other_user.login
                    ));
                    continue;
                }
                None => {
                    print_warning(&format!(
                        "{} Github sent an empty user; skipping as I can't make any assumptions here",
                        AUTHOR
                    ));
                    continue;
                }
            }

            let pr = client
                .pulls(&repo.owner, &repo.repo)
                .get(pull_request.number)
                .await
                .context("couldn't get details")?;

            let pr_head_ref = pr.head.sha;

            let checks = client
                .checks(&repo.owner, &repo.repo)
                .list_check_runs_for_git_ref(Commitish::from(pr_head_ref.clone()))
                .send()
                .await
                .context("couldn't get pr checks")?;

            let mut skip = false;
            for check in &checks.check_runs {
                match check.conclusion.as_deref() {
                    Some("success") => {
                        print_info(&format!("{} \"{}\": success", CHECK, check.name));
                    }
                    Some("skipped") => {
                        if config.merge_if_checks_skipped.unwrap_or(false) {
                            print_info(&format!("{} \"{}\": skipped", CHECK, check.name));
                        } else {
                            print_warning(&format!(
                                "{} \"{}\" skipped; merge_if_checks_skipped is false, so skipping",
                                CHECK, check.name
                            ));
                            skip = true;
                            break;
                        }
                    }
                    Some(non_successful_conclusion) => {
                        print_warning(&format!(
                            "{} \"{}\" {} ❌; skipping",
                            CHECK, check.name, non_successful_conclusion
                        ));
                        skip = true;
                        break;
                    }
                    None => {
                        print_warning(&format!(
                            "{} Github returned with an empty conclusion for the check {}; skipping as I can't make any assumptions here",
                            CHECK, check.name,
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
                        print_info(&format!("{} \"clean\"", STATE));
                    }
                    MergeableState::Blocked => {
                        if config.merge_if_blocked.unwrap_or(false) {
                            print_info(&format!(
                                "{} \"blocked\" (merge_if_blocked is true)",
                                STATE
                            ));
                        } else {
                            print_warning(&format!("{} blocked ❌; skipping", STATE));
                            continue;
                        }
                    }
                    other => {
                        print_warning(&format!("{} {:?} ❌; skipping", STATE, other));
                        continue;
                    }
                },
                None => {
                    print_warning(&format!(
                        "{} Github returned with an empty mergeable state; skipping as I can't make any assumptions here",
                        STATE
                    ));
                    continue;
                }
            }

            if !dry_run {
                print_info("PR matches all criteria, merging...");
                client
                    .pulls(&repo.owner, &repo.repo)
                    .merge(pr.number)
                    .method(config.merge_type.merge_method())
                    .send()
                    .await
                    .context("couldn't merge PR")?;
                print_success("PR merged! 🎉✅");

                break;
            } else {
                print_info("PR matches all criteria, would've been merged ✅");
            }
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

fn print_repo_info(name: &str) {
    println!(
        "{}",
        format!(
            r#"
========
{}
========"#,
            name
        )
        .bright_cyan()
    );
}

fn print_pr_info(msg: &str) {
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
