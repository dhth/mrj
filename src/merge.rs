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
const HEAD: &str = "[  head  ]  ";
const CHECK: &str = "[ check  ]  ";
const STATE: &str = "[ state  ]  ";

pub async fn merge_pr(client: Octocrab, config: Config, dry_run: bool) -> anyhow::Result<()> {
    print_banner(dry_run);

    println!("\n");
    if let Some(base_branch) = &config.base_branch {
        print_info(&format!(
            "I'm only looking for PRs where base branch is \"{}\"",
            base_branch
        ));
    } else {
        print_info("base_branch is not defined, I am not filtering PRs by base");
    }

    for repo in &config.repos {
        let pulls = client.pulls(&repo.owner, &repo.repo);

        let mut page_builder = pulls.list().state(State::Open).per_page(100);

        if let Some(base_branch) = &config.base_branch {
            page_builder = page_builder.base(base_branch);
        }

        let page = page_builder.send().await.context("couldn't get PRs")?;
        print_repo_info(&repo.repo);

        if page.items.is_empty() {
            println!();
            print_absence("no PRs");
            continue;
        }

        for pull_request in &page {
            print_pr_info(&format!(
                r#"
-> checking PR #{}
        {}
        {}"#,
                pull_request.number,
                pull_request.title.clone().unwrap_or_default(),
                pull_request
                    .html_url
                    .as_ref()
                    .map(|url| url.to_string())
                    .unwrap_or_default(),
            ));

            if let Some(head_pattern) = &config.head_pattern {
                if head_pattern.re.is_match(&pull_request.head.ref_field) {
                    print_qualification(&format!(
                        "{} \"{}\" matches the allowed head pattern",
                        HEAD, &pull_request.head.ref_field
                    ));
                } else {
                    print_disqualification(&format!(
                        "{} \"{}\" doesn't match the allowed head pattern",
                        HEAD, &pull_request.head.ref_field
                    ));
                    continue;
                }
            }

            match &pull_request.user {
                Some(trusted_user) if config.trusted_authors.contains(&trusted_user.login) => {
                    print_qualification(&format!(
                        "{} \"{}\" is in the list of trusted authors",
                        AUTHOR, trusted_user.login
                    ));
                }
                Some(other_user) => {
                    print_disqualification(&format!(
                        "{} \"{}\" is not in the list of trusted authors",
                        AUTHOR, other_user.login
                    ));
                    continue;
                }
                None => {
                    print_disqualification(&format!(
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
                        print_qualification(&format!("{} \"{}\": success", CHECK, check.name));
                    }
                    Some("skipped") => {
                        if config.merge_if_checks_skipped.unwrap_or(true) {
                            print_qualification(&format!("{} \"{}\": skipped", CHECK, check.name));
                        } else {
                            print_disqualification(&format!(
                                "{} \"{}\" skipped; merge_if_checks_skipped is false, so skipping",
                                CHECK, check.name
                            ));
                            skip = true;
                            break;
                        }
                    }
                    Some(non_successful_conclusion) => {
                        print_disqualification(&format!(
                            "{} \"{}\" {}; skipping",
                            CHECK, check.name, non_successful_conclusion
                        ));
                        skip = true;
                        break;
                    }
                    None => {
                        print_disqualification(&format!(
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

            match pr.mergeable_state.as_ref() {
                Some(state) => match state {
                    MergeableState::Clean => {
                        print_qualification(&format!("{} \"clean\"", STATE));
                    }
                    MergeableState::Blocked => {
                        if config.merge_if_blocked.unwrap_or(false) {
                            print_qualification(&format!(
                                "{} \"blocked\" (merge_if_blocked is true)",
                                STATE
                            ));
                        } else {
                            print_disqualification(&format!("{} blocked; skipping", STATE));
                            continue;
                        }
                    }
                    other => {
                        print_disqualification(&format!("{} {:?}; skipping", STATE, other));
                        continue;
                    }
                },
                None => {
                    print_disqualification(&format!(
                        "{} Github returned with an empty mergeable state; skipping as I can't make any assumptions here",
                        STATE
                    ));
                    continue;
                }
            }

            if dry_run {
                print_qualification(
                    "PR matches all criteria, I would've merged it if this weren't a dry run ✅",
                );
            } else {
                print_qualification("PR matches all criteria, merging...");
                client
                    .pulls(&repo.owner, &repo.repo)
                    .merge(pr.number)
                    .method(config.merge_type.merge_method())
                    .send()
                    .await
                    .context("couldn't merge PR")?;
                print_success("PR merged! 🎉✅");

                break;
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

fn print_info(message: &str) {
    println!("[INFO] {}", message);
}

fn print_repo_info(name: &str) {
    println!(
        "{}",
        format!(
            r#"

=============
{}
============="#,
            name
        )
        .cyan()
    );
}

fn print_pr_info(msg: &str) {
    println!("{}", msg.purple());
}

fn print_qualification(msg: &str) {
    println!("        {}", msg.blue());
}

fn print_disqualification(msg: &str) {
    println!("        {} ❌", msg.yellow());
}

fn print_absence(msg: &str) {
    println!("        {}", msg.yellow());
}

fn print_success(msg: &str) {
    println!("        {}", msg.green());
}
