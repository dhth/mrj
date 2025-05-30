use crate::domain::{
    Disqualification, MergeResult, MergedPR, Qualification, RepoResult, RunSummary,
};
use anyhow::Context;
use colored::Colorize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const BANNER: &str = include_str!("assets/banner.txt");
const AUTHOR: &str = "[ author ]  ";
const HEAD: &str = "[ head   ]  ";
const CHECK: &str = "[ check  ]  ";
const STATE: &str = "[ state  ]  ";

pub(super) struct RunLog {
    write_to_file: bool,
    lines: Vec<String>,
    summary: RunSummary,
    ignore_repos_with_no_prs: bool,
    dry_run: bool,
}

impl RunLog {
    pub(super) fn new(output: bool, ignore_repos_with_no_prs: bool, dry_run: bool) -> Self {
        RunLog {
            write_to_file: output,
            lines: vec![],
            summary: RunSummary::default(),
            ignore_repos_with_no_prs,
            dry_run,
        }
    }

    pub(super) fn add_repo_result(&mut self, result: RepoResult) {
        self.summary.record_repo();

        match &result {
            RepoResult::Errored(repo_check) => self.error(repo_check.state.reason()),
            RepoResult::Finished(repo_check) => {
                if repo_check.results().is_empty() {
                    self.summary.record_repo_with_no_count();
                    if self.ignore_repos_with_no_prs {
                        return;
                    }
                }

                let repo = &result.name();
                self.repo_info(repo);

                if repo_check.results().is_empty() {
                    self.empty_line();
                    self.absence("no PRs");
                    return;
                }

                repo_check
                    .results()
                    .iter()
                    .for_each(|r| self.add_merge_result(r, repo));
            }
        }
    }

    pub(super) fn write_output<P>(
        &mut self,
        output_to_file: bool,
        output_path: P,
        summary_to_file: bool,
        summary_path: P,
    ) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let prs_merged = if self.summary.prs_merged.is_empty() {
            None
        } else {
            Some(format!(
                r#"

PRs merged
---

{}"#,
                self.summary
                    .prs_merged
                    .iter()
                    .map(|pr| format!("- [{}] {}", pr.repo, pr.title))
                    .collect::<Vec<String>>()
                    .join("\n"),
            ))
        };

        let summary = format!(
            r#"

===========
  SUMMARY
===========

# PRs merged          :  {}
# PRs disqualified    :  {}
# Repos checked       :  {}
# Repos with no PRs   :  {}
# Errors encountered  :  {}{}"#,
            self.summary.prs_merged.len(),
            self.summary.num_disqualifications,
            self.summary.num_repos,
            self.summary.num_repos_with_no_prs,
            self.summary.num_errors,
            prs_merged.unwrap_or_default(),
        );

        println!("{}", &summary.green());

        if !output_to_file {
            return Ok(());
        }

        self.lines.push(summary.clone());

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path)
            .context("couldn't open a handle to the output file")?;

        file.write_all(self.lines.join("\n").as_bytes())
            .context("couldn't write output to file")?;

        if summary_to_file {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(summary_path)
                .context("couldn't open a handle to the summary file")?;

            file.write_all(summary.trim_start().as_bytes())
                .context("couldn't write output to file")?;
        }

        Ok(())
    }

    pub(super) fn banner(&mut self) {
        println!("{}", BANNER.green().bold());
        if self.dry_run {
            println!("{}", "                         dry run".yellow());
        }
        println!("\n");

        if self.write_to_file {
            self.lines.push(BANNER.to_string());
            if self.dry_run {
                self.lines
                    .push("                         dry run".to_string());
            }
            self.lines.push("\n".to_string());
        }
    }

    pub(super) fn info(&mut self, message: &str) {
        println!("[INFO] {}", message);

        if self.write_to_file {
            self.lines.push(format!("[INFO] {}", message));
        }
    }

    pub(super) fn empty_line(&mut self) {
        println!();

        if self.write_to_file {
            self.lines.push("".to_string());
        }
    }

    fn add_merge_result(&mut self, result: &MergeResult, repo: &str) {
        self.pr_info(&format!(
            r#"
-> checking PR #{}
        {}
        {}"#,
            result.pr_number(),
            result.pr_title(),
            result.pr_url(),
        ));

        for q in result.qualifications() {
            self.qualification(q);
        }

        match result {
            MergeResult::Disqualified(pr_check) => {
                self.disqualification(pr_check.state.reason());
            }
            MergeResult::Errored(pr_check) => {
                self.error(pr_check.state.reason());
            }
            MergeResult::Qualified(_) => {
                let merged_pr = MergedPR {
                    repo: repo.to_string(),
                    title: result.pr_title().to_string(),
                };
                self.merge(merged_pr);
            }
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

        if self.write_to_file {
            self.lines.push(format!(
                r#"

=============
  {}
============="#,
                name
            ));
        }
    }

    fn pr_info(&mut self, msg: &str) {
        println!("{}", msg.purple());

        if self.write_to_file {
            self.lines.push(msg.to_string());
        }
    }

    fn qualification(&mut self, q: &Qualification) {
        let msg = match q {
            Qualification::Head(h) => {
                format!("{} \"{}\" matches the allowed head pattern", HEAD, h)
            }
            Qualification::User(a) => {
                format!("{} \"{}\" is in the list of trusted authors", AUTHOR, a)
            }
            Qualification::Check { name, conclusion } => {
                format!(
                    "{} \"{}\" concluded with desired status: \"{}\"",
                    CHECK, name, conclusion,
                )
            }
            Qualification::State(s) => format!("{} \"{}\" is desirable", STATE, s),
        };

        println!("        {}", msg.blue());

        if self.write_to_file {
            self.lines.push(format!("        {}", msg));
        }
    }

    fn disqualification(&mut self, dq: &Disqualification) {
        let msg = match dq {
            Disqualification::Head(h) => {
                format!("{} \"{}\" doesn't match the allowed head pattern", HEAD, h)
            }
            Disqualification::User(maybe_author) => match maybe_author {
                Some(a) => format!("{} \"{}\" is not in the list of trusted authors", AUTHOR, a),
                None => format!(
                    "{} Github sent an empty user; skipping as I can't make any assumptions here",
                    AUTHOR
                ),
            },
            Disqualification::Check { name, conclusion } => match conclusion {
                Some(c) => format!(
                    "{} \"{}\" concluded with undesired status: \"{}\"",
                    CHECK, name, c
                ),
                None => format!(
                    "{} Github returned with an empty conclusion for the check {}; skipping as I can't make any assumptions here",
                    CHECK, name,
                ),
            },
            Disqualification::State(maybe_state) => match maybe_state {
                Some(s) => format!("{} \"{}\" is undesirable", STATE, s),
                None => format!(
                    "{} Github returned with an empty mergeable state; skipping as I can't make any assumptions here",
                    STATE
                ),
            },
        };

        println!("        {} ❌", msg.yellow());

        if self.write_to_file {
            self.lines.push(format!("        {} ❌", msg));
        }

        self.summary.record_disqualification();
    }

    fn absence(&mut self, msg: &str) {
        println!("        {}", msg.yellow());

        if self.write_to_file {
            self.lines.push(format!("        {}", msg));
        }
    }

    fn merge(&mut self, pr: MergedPR) {
        let msg = if self.dry_run {
            "PR matches all criteria, I would've merged it if this weren't a dry run ✅"
        } else {
            "PR merged! 🎉 ✅"
        };

        println!("        {}", msg.green());

        if self.write_to_file {
            self.lines.push(format!("        {}", msg));
        }

        if !self.dry_run {
            self.summary.record_merged_pr(pr);
        }
    }

    fn error(&mut self, error: &anyhow::Error) {
        println!("{}", format!("        error 😵: {}", error).red());

        if self.write_to_file {
            self.lines.push(format!("        error 😵: {}", error));
        }

        self.summary.record_error();
    }
}
