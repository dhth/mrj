use super::behaviours::RunBehaviours;
use crate::domain::{
    Disqualification, MergeResult, MergedPR, Qualification, RepoResult, RunSummary,
};
use anyhow::Context;
use colored::Colorize;
use std::fs::OpenOptions;
use std::io::Write;

const BANNER: &str = include_str!("assets/banner.txt");
const AUTHOR: &str = "[ author ]  ";
const HEAD: &str = "[ head  ]  ";
const CHECK: &str = "[ check  ]  ";
const STATE: &str = "[ state  ]  ";

pub(super) struct RunLog<W: Write> {
    w: W,
    behaviours: RunBehaviours,
    lines: Vec<String>,
    summary: RunSummary,
}

impl<W: Write> RunLog<W> {
    pub(super) fn new(writer: W, behaviours: &RunBehaviours) -> Self {
        RunLog {
            w: writer,
            behaviours: behaviours.clone(),
            lines: vec![],
            summary: RunSummary::default(),
        }
    }

    pub(super) fn add_repo_result(&mut self, result: RepoResult) {
        self.summary.record_repo();

        match &result {
            RepoResult::Errored(repo_check) => {
                self.repo_info(&result.name());
                self.empty_line();
                self.error(repo_check.state.reason());
            }
            RepoResult::Finished(repo_check) => {
                let filtered_results = repo_check
                    .results()
                    .iter()
                    .filter(|result| match result {
                        MergeResult::Disqualified(pr_check) => match pr_check.state.reason() {
                            Disqualification::Author(_)
                                if !self.behaviours.show_prs_from_untrusted_authors =>
                            {
                                false
                            }
                            Disqualification::Head(_)
                                if !self.behaviours.show_prs_with_unmatched_head =>
                            {
                                false
                            }
                            _ => true,
                        },
                        _ => true,
                    })
                    .collect::<Vec<_>>();

                if filtered_results.is_empty() {
                    self.summary.record_repo_with_no_prs();
                    if !self.behaviours.show_repos_with_no_prs {
                        return;
                    }
                }

                let repo = &result.name();
                self.repo_info(repo);

                if filtered_results.is_empty() {
                    self.empty_line();
                    self.absence("no PRs");
                    return;
                }

                filtered_results
                    .iter()
                    .for_each(|r| self.add_merge_result(r, repo));
            }
        }
    }

    pub(super) fn write_output(&mut self) -> anyhow::Result<()> {
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

        let disqualifications_summary = if !self.behaviours.skip_disqualifications_in_summary
            && !self.summary.disqualifications.is_empty()
        {
            let longest_url_len = self
                .summary
                .disqualifications
                .iter()
                .map(|(p, _)| p.len())
                .max()
                .unwrap_or(80);

            Some(format!(
                r#"

Disqualifications
---

{}"#,
                self.summary
                    .disqualifications
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
- Repos checked:                 {}
- Repos with no relevant PRs:    {}
- Errors encountered:            {}{}{}"#,
            self.summary.prs_merged.len(),
            self.summary.disqualifications.len(),
            self.summary.num_repos,
            self.summary.num_repos_with_no_prs,
            self.summary.num_errors,
            prs_merged.unwrap_or_default(),
            disqualifications_summary.unwrap_or_default(),
        );

        let output = if self.behaviours.plain_stdout {
            &summary
        } else {
            &summary.green().to_string()
        };

        let _ = writeln!(self.w, "{output}");

        if let Some(output_path) = &self.behaviours.output_path {
            self.lines.push(summary.clone());

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(output_path.as_path())
                .context("couldn't open a handle to the output file")?;

            file.write_all(self.lines.join("\n").as_bytes())
                .context("couldn't write output to file")?;
        }

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

    pub(super) fn banner(&mut self) {
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

        if self.behaviours.output_path.is_some() {
            self.lines.push(BANNER.to_string());
            if !self.behaviours.execute {
                self.lines.push(dry_run_line);
            }
            self.lines.push("".to_string());
        }
    }

    pub(super) fn info(&mut self, message: &str) {
        let _ = writeln!(self.w, "[INFO] {message}");

        if self.behaviours.output_path.is_some() {
            self.lines.push(format!("[INFO] {message}"));
        }
    }

    pub(super) fn empty_line(&mut self) {
        let _ = writeln!(self.w);

        if self.behaviours.output_path.is_some() {
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
                self.disqualification(&pr_check.url, pr_check.state.reason());
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

        if self.behaviours.output_path.is_some() {
            self.lines.push(line);
        }
    }

    fn pr_info(&mut self, msg: &str) {
        let output = if self.behaviours.plain_stdout {
            msg
        } else {
            &msg.purple().to_string()
        };

        let _ = writeln!(self.w, "{output}");

        if self.behaviours.output_path.is_some() {
            self.lines.push(msg.to_string());
        }
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

        if self.behaviours.output_path.is_some() {
            self.lines.push(format!("        {msg}"));
        }
    }

    fn disqualification(&mut self, pr_url: &str, dq: &Disqualification) {
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

        if self.behaviours.output_path.is_some() {
            self.lines.push(format!("        {msg} ❌"));
        }

        self.summary.record_disqualification(pr_url, dq);
    }

    fn absence(&mut self, msg: &str) {
        let output = if self.behaviours.plain_stdout {
            msg
        } else {
            &msg.yellow().to_string()
        };

        let _ = writeln!(self.w, "        {output}");

        if self.behaviours.output_path.is_some() {
            self.lines.push(format!("        {msg}"));
        }
    }

    fn merge(&mut self, pr: MergedPR) {
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

        if self.behaviours.output_path.is_some() {
            self.lines.push(format!("        {msg}"));
        }

        if self.behaviours.execute {
            self.summary.record_merged_pr(pr);
        }
    }

    fn error(&mut self, error: &anyhow::Error) {
        let line = format!("        error 😵: {error}");
        let output = if self.behaviours.plain_stdout {
            &line
        } else {
            &line.red().to_string()
        };

        let _ = writeln!(self.w, "{output}");

        if self.behaviours.output_path.is_some() {
            self.lines.push(line);
        }

        self.summary.record_error();
    }
}
