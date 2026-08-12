use super::behaviours::RunBehaviours;
use crate::config::Config;
use crate::domain::{
    Disqualification, GhApiQueryParam, MergeResult, Qualification, RepoResult, RunSummary,
};
use anyhow::Context;
use chrono::{DateTime, Utc};
use colored::Colorize;
use std::fs::OpenOptions;
use std::io::Write;

const BANNER: &str = include_str!("assets/banner.txt");
const AUTHOR: &str = "[ author ]  ";
const HEAD: &str = "[ head  ]  ";
const CHECK: &str = "[ check  ]  ";
const STATE: &str = "[ state  ]  ";

pub(super) struct RunLogger<W: Write> {
    w: W,
    behaviours: RunBehaviours,
}

impl<W: Write> RunLogger<W> {
    pub(super) fn new(writer: W, behaviours: &RunBehaviours) -> Self {
        RunLogger {
            w: writer,
            behaviours: behaviours.clone(),
        }
    }

    pub(super) fn add_repo_result(&mut self, result: &RepoResult) {
        match &result {
            RepoResult::Errored(repo_check) => {
                self.repo_info(&result.name());
                self.empty_line();
                self.error(repo_check.state.reason());
            }
            RepoResult::Finished(repo_check) => {
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
                    .for_each(|r| self.add_merge_result(r));
            }
        }
    }

    pub(super) fn write_output(&mut self, summary: &RunSummary) -> anyhow::Result<()> {
        let prs_merged = if summary.prs_merged.is_empty() {
            None
        } else {
            Some(format!(
                r#"

PRs merged
---

{}"#,
                summary
                    .prs_merged
                    .iter()
                    .map(|pr| format!("- [{}] {}", pr.repo, pr.title))
                    .collect::<Vec<String>>()
                    .join("\n"),
            ))
        };

        let disqualifications_summary = if !self.behaviours.skip_disqualifications_in_summary
            && !summary.disqualifications.is_empty()
        {
            let longest_url_len = self
                .disqualifications(summary)
                .iter()
                .map(|(p, _)| p.len())
                .max()
                .unwrap_or(80);

            Some(format!(
                r#"

Disqualifications
---

{}"#,
                self.disqualifications(summary)
                    .iter()
                    .map(|(u, d)| format!("- {u:<longest_url_len$}        {d}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ))
        } else {
            None
        };

        let summary = format!(
            r#"
===========
  SUMMARY
===========

- PRs merged:                    {}
- PRs disqualified:              {}
- Errors encountered:            {}{}{}"#,
            summary.prs_merged.len(),
            summary.disqualifications.len(),
            summary.num_errors,
            prs_merged.unwrap_or_default(),
            disqualifications_summary.unwrap_or_default(),
        );

        let output = if self.behaviours.plain_stdout {
            &summary
        } else {
            &summary.green().to_string()
        };

        let _ = writeln!(self.w, "{output}");

        if let Some(summary_path) = &self.behaviours.summary_path {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(summary_path.as_path())
                .context("couldn't open a handle to the summary file")?;

            file.write_all(summary.trim_start().as_bytes())
                .context("couldn't write output to file")?;
        }

        Ok(())
    }

    pub(super) fn print_banner(&mut self) {
        let banner_output = if self.behaviours.plain_stdout {
            BANNER
        } else {
            &BANNER.green().bold().to_string()
        };

        let _ = writeln!(self.w, "{banner_output}");

        let dry_run_line = "                         dry run".to_string();
        let dry_run_output = if self.behaviours.plain_stdout {
            &dry_run_line
        } else {
            &dry_run_line.yellow().to_string()
        };

        if !self.behaviours.execute {
            let _ = writeln!(self.w, "{dry_run_output}");
        }

        let _ = writeln!(self.w);
    }

    fn disqualifications<'a>(&self, summary: &'a RunSummary) -> Vec<(&'a str, &'a str)> {
        summary
            .disqualifications
            .iter()
            .map(|dq| (dq.pr_url.as_str(), dq.reason.as_str()))
            .collect()
    }

    pub(super) fn print_startup_info(&mut self, config: &Config, now: DateTime<Utc>) {
        self.info(&format!("The time right now is {now}"));

        if let Some(b) = &config.base_branch {
            self.info(&format!(
                "I'm only looking for PRs where the base branch is \"{b}\""
            ));
        }

        if config.merge_if_blocked {
            self.info("I will merge PRs even if they're blocked");
        }

        if !config.merge_if_checks_skipped {
            self.info("I won't merge PRs if checks are skipped");
        }

        if config.merge_if_checks_neutral {
            self.info("I will merge PRs if checks conclude with a neutral status");
        }

        if self.behaviours.show_repos_with_no_prs {
            self.info("I will show repositories that have no PRs");
        }

        if self.behaviours.show_prs_from_untrusted_authors {
            self.info("I will show PRs from untrusted authors");
        }

        if self.behaviours.show_prs_with_unmatched_head && config.head_pattern.is_some() {
            self.info("I will show PRs from where head doesn't match configured head pattern");
        }

        self.info(&format!(
            r#"I'm sorting PRs based on "{}" in the "{}" direction"#,
            config.sort_by.readable_repr(),
            config.sort_direction.readable_repr()
        ));
    }

    pub(super) fn print_conclusion(&mut self, now: DateTime<Utc>, num_seconds: i64) {
        self.empty_line();
        self.info(&format!(
            "This run ended at {now}; took {num_seconds} seconds"
        ));
    }

    fn info(&mut self, message: &str) {
        let _ = writeln!(self.w, "[INFO] {message}");
    }

    fn empty_line(&mut self) {
        let _ = writeln!(self.w);
    }

    fn add_merge_result(&mut self, result: &MergeResult) {
        self.pr_info(&format!(
            r#"
-> checking PR #{}
        {}
        {}"#,
            result.pr_number(),
            result.pr_title(),
            result.pr_url(),
        ));

        match (result.pr_created_at(), result.pr_updated_at()) {
            (None, None) => {}
            (None, Some(_)) => {}
            (Some(c), None) => {
                self.pr_info(&format!("        Created: {}", c.to_rfc2822()));
            }
            (Some(c), Some(u)) if c == u => {
                self.pr_info(&format!("        Created: {}", c.to_rfc2822()))
            }
            (Some(c), Some(u)) => {
                self.pr_info(&format!("        Created: {}", c.to_rfc2822()));
                self.pr_info(&format!("        Updated: {}", u.to_rfc2822()));
            }
        };

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
                self.merge();
            }
        }
    }

    fn repo_info(&mut self, name: &str) {
        let line = format!(
            r#"

=============
  {name}
============="#
        );

        let output = if self.behaviours.plain_stdout {
            &line
        } else {
            &line.cyan().to_string()
        };

        let _ = writeln!(self.w, "{output}");
    }

    fn pr_info(&mut self, msg: &str) {
        let output = if self.behaviours.plain_stdout {
            msg
        } else {
            &msg.purple().to_string()
        };

        let _ = writeln!(self.w, "{output}");
    }

    fn qualification(&mut self, q: &Qualification) {
        let msg = match q {
            Qualification::Head(h) => {
                format!("{HEAD} \"{h}\" matches the allowed head pattern")
            }
            Qualification::Author(a) => {
                format!("{AUTHOR} \"{a}\" is in the list of trusted authors")
            }
            Qualification::Check { name, conclusion } => {
                format!("{CHECK} \"{name}\" concluded with desired status: \"{conclusion}\"",)
            }
            Qualification::State(s) => format!("{STATE} \"{s}\" is desirable"),
        };

        let output = if self.behaviours.plain_stdout {
            &msg
        } else {
            &msg.blue().to_string()
        };

        let _ = writeln!(self.w, "        {output}");
    }

    fn disqualification(&mut self, dq: &Disqualification) {
        let msg = match dq {
            Disqualification::Head(h) => {
                format!("{HEAD} \"{h}\" doesn't match the allowed head pattern")
            }
            Disqualification::Author(maybe_author) => match maybe_author {
                Some(a) => format!("{AUTHOR} \"{a}\" is not in the list of trusted authors"),
                None => format!(
                    "{AUTHOR} Github sent an empty user; skipping as I can't make any assumptions here"
                ),
            },
            Disqualification::Check { name, conclusion } => match conclusion {
                Some(c) => format!("{CHECK} \"{name}\" concluded with undesired status: \"{c}\""),
                None => format!(
                    "{CHECK} Github returned with an empty conclusion for the check {name}; skipping as I can't make any assumptions here",
                ),
            },
            Disqualification::State(maybe_state) => match maybe_state {
                Some(s) => format!("{STATE} \"{s}\" is undesirable"),
                None => format!(
                    "{STATE} Github returned with an empty mergeable state; skipping as I can't make any assumptions here"
                ),
            },
        };

        let output = if self.behaviours.plain_stdout {
            &msg
        } else {
            &msg.yellow().to_string()
        };

        let _ = writeln!(self.w, "        {output} ❌");
    }

    fn absence(&mut self, msg: &str) {
        let output = if self.behaviours.plain_stdout {
            msg
        } else {
            &msg.yellow().to_string()
        };

        let _ = writeln!(self.w, "        {output}");
    }

    fn merge(&mut self) {
        let msg = if self.behaviours.execute {
            "PR merged! 🎉 ✅"
        } else {
            "PR matches all criteria, I would've merged it if this weren't a dry run ✅"
        };

        let output = if self.behaviours.plain_stdout {
            msg
        } else {
            &msg.green().to_string()
        };

        let _ = writeln!(self.w, "        {output}");
    }

    fn error(&mut self, error: &anyhow::Error) {
        let line = format!("        error 😵: {error:#}");
        let output = if self.behaviours.plain_stdout {
            &line
        } else {
            &line.red().to_string()
        };

        let _ = writeln!(self.w, "{output}");
    }
}
