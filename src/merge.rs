use std::path::Path;

use crate::config::Config;
use crate::domain::Repo;
use anyhow::Context;
use chrono::Utc;
use colored::Colorize;
use octocrab::Octocrab;
use octocrab::{
    models::pulls::MergeableState,
    params::{State, repos::Commitish},
};
use std::fs::{File, OpenOptions};
use std::io::Write;

const BANNER: &str = include_str!("assets/banner.txt");
const AUTHOR: &str = "[ author ]  ";
const HEAD: &str = "[ head   ]  ";
const CHECK: &str = "[ check  ]  ";
const STATE: &str = "[ state  ]  ";

pub async fn merge_prs<P>(
    client: Octocrab,
    config: Config,
    repos_override: Vec<Repo>,
    output: bool,
    output_file: P,
    dry_run: bool,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let out_file = if output {
        Some(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(output_file)
                .context("couldn't open a handle to the output file")?,
        )
    } else {
        None
    };

    let mut p = Printer::new(out_file);
    p.banner(dry_run);

    let start = Utc::now();
    p.info(&format!("The time right now is {}", start));

    if let Some(base_branch) = &config.base_branch {
        p.info(&format!(
            "I'm only looking for PRs where base branch is \"{}\"",
            base_branch
        ));
    } else {
        p.info("base_branch is not defined, I am not filtering PRs by base");
    }

    let repos_to_use = if repos_override.is_empty() {
        &config.repos
    } else {
        &repos_override
    };

    for repo in repos_to_use {
        let pulls = client.pulls(&repo.owner, &repo.repo);

        let mut page_builder = pulls.list().state(State::Open).per_page(100);

        if let Some(base_branch) = &config.base_branch {
            page_builder = page_builder.base(base_branch);
        }

        let page = page_builder.send().await.context("couldn't get PRs")?;
        p.repo_info(&repo.repo);

        if page.items.is_empty() {
            println!();
            p.absence("no PRs");
            continue;
        }

        for pull_request in &page {
            p.pr_info(&format!(
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
                    p.qualification(&format!(
                        "{} \"{}\" matches the allowed head pattern",
                        HEAD, &pull_request.head.ref_field
                    ));
                } else {
                    p.disqualification(&format!(
                        "{} \"{}\" doesn't match the allowed head pattern",
                        HEAD, &pull_request.head.ref_field
                    ));
                    continue;
                }
            }

            match &pull_request.user {
                Some(trusted_user) if config.trusted_authors.contains(&trusted_user.login) => {
                    p.qualification(&format!(
                        "{} \"{}\" is in the list of trusted authors",
                        AUTHOR, trusted_user.login
                    ));
                }
                Some(other_user) => {
                    p.disqualification(&format!(
                        "{} \"{}\" is not in the list of trusted authors",
                        AUTHOR, other_user.login
                    ));
                    continue;
                }
                None => {
                    p.disqualification(
                        &format!(
                            "{} Github sent an empty user; skipping as I can't make any assumptions here",
                            AUTHOR
                        ),
                    );
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
                        p.qualification(&format!("{} \"{}\": success", CHECK, check.name));
                    }
                    Some("skipped") => {
                        if config.merge_if_checks_skipped.unwrap_or(true) {
                            p.qualification(&format!("{} \"{}\": skipped", CHECK, check.name));
                        } else {
                            p.disqualification(&format!(
                                "{} \"{}\" skipped; merge_if_checks_skipped is false, so skipping",
                                CHECK, check.name
                            ));
                            skip = true;
                            break;
                        }
                    }
                    Some(non_successful_conclusion) => {
                        p.disqualification(&format!(
                            "{} \"{}\" {}; skipping",
                            CHECK, check.name, non_successful_conclusion
                        ));
                        skip = true;
                        break;
                    }
                    None => {
                        p.disqualification(
                            &format!(
                                "{} Github returned with an empty conclusion for the check {}; skipping as I can't make any assumptions here",
                                CHECK, check.name,
                            ),
                        );
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
                        p.qualification(&format!("{} \"clean\"", STATE));
                    }
                    MergeableState::Blocked => {
                        if config.merge_if_blocked.unwrap_or(false) {
                            p.qualification(&format!(
                                "{} \"blocked\" (merge_if_blocked is true)",
                                STATE
                            ));
                        } else {
                            p.disqualification(&format!("{} blocked; skipping", STATE));
                            continue;
                        }
                    }
                    other => {
                        p.disqualification(&format!("{} {:?}; skipping", STATE, other));
                        continue;
                    }
                },
                None => {
                    p.disqualification(
                        &format!(
                            "{} Github returned with an empty mergeable state; skipping as I can't make any assumptions here",
                            STATE
                        ),
                    );
                    continue;
                }
            }

            if dry_run {
                p.qualification(
                    "PR matches all criteria, I would've merged it if this weren't a dry run ✅",
                );
            } else {
                p.qualification("PR matches all criteria, merging...");
                client
                    .pulls(&repo.owner, &repo.repo)
                    .merge(pr.number)
                    .method(config.merge_type.merge_method())
                    .send()
                    .await
                    .context("couldn't merge PR")?;
                p.success("PR merged! 🎉 ✅");

                break;
            }
        }
    }

    let end_ts = Utc::now();
    let num_seconds = (end_ts - start).num_seconds();

    p.empty_line();
    p.info(&format!(
        "This run ended at {}; took {} seconds",
        end_ts, num_seconds
    ));

    Ok(())
}

struct Printer {
    out_file: Option<File>,
}

impl Printer {
    fn new(out_file: Option<File>) -> Self {
        Printer { out_file }
    }

    fn banner(&mut self, dry_run: bool) {
        println!("{}", BANNER.green().bold());
        if dry_run {
            println!("{}", "                         dry run".yellow());
        }
        println!("\n");

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o, "{}", BANNER);
            if dry_run {
                let _ = writeln!(o, "                         dry run");
            }
            let _ = writeln!(o, "\n");
        }
    }

    fn info(&mut self, message: &str) {
        println!("[INFO] {}", message);

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o, "[INFO] {}", message);
        }
    }

    fn repo_info(&mut self, name: &str) {
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

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(
                o,
                r#"

=============
  {}
============="#,
                name
            );
        }
    }

    fn pr_info(&mut self, msg: &str) {
        println!("{}", msg.purple());

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o, "{}", msg);
        }
    }

    fn qualification(&mut self, msg: &str) {
        println!("        {}", msg.blue());

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o, "        {}", msg);
        }
    }

    fn disqualification(&mut self, msg: &str) {
        println!("        {} ❌", msg.yellow());

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o, "        {} ❌", msg);
        }
    }

    fn absence(&mut self, msg: &str) {
        println!("        {}", msg.yellow());

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o, "        {}", msg);
        }
    }

    fn success(&mut self, msg: &str) {
        println!("        {}", msg.green());

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o, "        {}", msg);
        }
    }

    fn empty_line(&mut self) {
        println!();

        if let Some(o) = self.out_file.as_mut() {
            let _ = writeln!(o);
        }
    }
}
