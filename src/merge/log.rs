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
const HEAD: &str = "[ head   ]  ";
const CHECK: &str = "[ check  ]  ";
const STATE: &str = "[ state  ]  ";

pub(super) struct RunLog<W: Write> {
    w: W,
    b: RunBehaviours,
    lines: Vec<String>,
    summary: RunSummary,
}

impl<W: Write> RunLog<W> {
    pub(super) fn new(writer: W, behaviours: &RunBehaviours) -> Self {
        RunLog {
            w: writer,
            b: behaviours.clone(),
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
                                if !self.b.show_prs_from_untrusted_authors =>
                            {
                                false
                            }
                            Disqualification::Head(_) if !self.b.show_prs_with_unmatched_head => {
                                false
                            }
                            _ => true,
                        },
                        _ => true,
                    })
                    .collect::<Vec<_>>();

                if filtered_results.is_empty() {
                    self.summary.record_repo_with_no_prs();
                    if !self.b.show_repos_with_no_prs {
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

        let disqualifications_summary =
            if self.b.summarize_disqualifications && !self.summary.disqualifications.is_empty() {
                Some(format!(
                    r#"

Disqualifications
---

{}"#,
                    self.summary
                        .disqualifications
                        .iter()
                        .map(|d| format!("- {}", d))
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

# PRs merged                  :  {}
# PRs disqualified            :  {}
# Repos checked               :  {}
# Repos with no relevant PRs  :  {}
# Errors encountered          :  {}{}{}"#,
            self.summary.prs_merged.len(),
            self.summary.disqualifications.len(),
            self.summary.num_repos,
            self.summary.num_repos_with_no_prs,
            self.summary.num_errors,
            prs_merged.unwrap_or_default(),
            disqualifications_summary.unwrap_or_default(),
        );

        let output = if self.b.plain_stdout {
            &summary
        } else {
            &summary.green().to_string()
        };

        let _ = writeln!(self.w, "{}", output);

        if let Some(output_path) = &self.b.output_path {
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

        if let Some(summary_path) = &self.b.summary_path {
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
        let banner_output = if self.b.plain_stdout {
            BANNER
        } else {
            &BANNER.green().bold().to_string()
        };

        let _ = writeln!(self.w, "{}", banner_output);

        let dry_run_line = "                         dry run".to_string();
        let dry_run_output = if self.b.plain_stdout {
            &dry_run_line
        } else {
            &dry_run_line.yellow().to_string()
        };

        if !self.b.execute {
            let _ = writeln!(self.w, "{}", dry_run_output);
        }

        let _ = writeln!(self.w);

        if self.b.output_path.is_some() {
            self.lines.push(BANNER.to_string());
            if !self.b.execute {
                self.lines.push(dry_run_line);
            }
            self.lines.push("".to_string());
        }
    }

    pub(super) fn info(&mut self, message: &str) {
        let _ = writeln!(self.w, "[INFO] {}", message);

        if self.b.output_path.is_some() {
            self.lines.push(format!("[INFO] {}", message));
        }
    }

    pub(super) fn empty_line(&mut self) {
        let _ = writeln!(self.w);

        if self.b.output_path.is_some() {
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
  {}
============="#,
            name
        );

        let output = if self.b.plain_stdout {
            &line
        } else {
            &line.cyan().to_string()
        };

        let _ = writeln!(self.w, "{}", output);

        if self.b.output_path.is_some() {
            self.lines.push(line);
        }
    }

    fn pr_info(&mut self, msg: &str) {
        let output = if self.b.plain_stdout {
            msg
        } else {
            &msg.purple().to_string()
        };

        let _ = writeln!(self.w, "{}", output);

        if self.b.output_path.is_some() {
            self.lines.push(msg.to_string());
        }
    }

    fn qualification(&mut self, q: &Qualification) {
        let msg = match q {
            Qualification::Head(h) => {
                format!("{} \"{}\" matches the allowed head pattern", HEAD, h)
            }
            Qualification::Author(a) => {
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

        let output = if self.b.plain_stdout {
            &msg
        } else {
            &msg.blue().to_string()
        };

        let _ = writeln!(self.w, "        {}", output);

        if self.b.output_path.is_some() {
            self.lines.push(format!("        {}", msg));
        }
    }

    fn disqualification(&mut self, pr_url: &str, dq: &Disqualification) {
        let msg = match dq {
            Disqualification::Head(h) => {
                format!("{} \"{}\" doesn't match the allowed head pattern", HEAD, h)
            }
            Disqualification::Author(maybe_author) => match maybe_author {
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

        let output = if self.b.plain_stdout {
            &msg
        } else {
            &msg.yellow().to_string()
        };

        let _ = writeln!(self.w, "        {} ❌", output);

        if self.b.output_path.is_some() {
            self.lines.push(format!("        {} ❌", msg));
        }

        self.summary.record_disqualification(pr_url, dq);
    }

    fn absence(&mut self, msg: &str) {
        let output = if self.b.plain_stdout {
            msg
        } else {
            &msg.yellow().to_string()
        };

        let _ = writeln!(self.w, "        {}", output);

        if self.b.output_path.is_some() {
            self.lines.push(format!("        {}", msg));
        }
    }

    fn merge(&mut self, pr: MergedPR) {
        let msg = if self.b.execute {
            "PR merged! 🎉 ✅"
        } else {
            "PR matches all criteria, I would've merged it if this weren't a dry run ✅"
        };

        let output = if self.b.plain_stdout {
            msg
        } else {
            &msg.green().to_string()
        };

        let _ = writeln!(self.w, "        {}", output);

        if self.b.output_path.is_some() {
            self.lines.push(format!("        {}", msg));
        }

        if self.b.execute {
            self.summary.record_merged_pr(pr);
        }
    }

    fn error(&mut self, error: &anyhow::Error) {
        let line = format!("        error 😵: {}", error);
        let output = if self.b.plain_stdout {
            &line
        } else {
            &line.red().to_string()
        };

        let _ = writeln!(self.w, "{}", output);

        if self.b.output_path.is_some() {
            self.lines.push(line);
        }

        self.summary.record_error();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        PRCheck, PRCheckFinished, PRDisqualified, RepoCheck, RepoCheckErrored, RepoCheckFinished,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use pretty_assertions::assert_eq;

    const OWNER: &str = "dhth";
    const REPO: &str = "mrj";
    const PR_TITLE: &str = "build: bump clap from 4.5.39 to 4.5.40";
    const PR_URL: &str = "https://github.com/dhth/mrj/pull/1";
    const PR_HEAD: &str = "dependabot/cargo/clap-4.5.40";
    const PR_AUTHOR: &str = "dependabot[bot]";

    #[test]
    fn failed_repo_result_is_printed_correctly() {
        // GIVEN
        let mut buffer = vec![];
        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckErrored(anyhow::anyhow!("something went wrong")),
        };
        let repo_result = RepoResult::Errored(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

        error 😵: something went wrong
"#
        );
    }

    #[test]
    fn pr_with_unmatched_head_is_ignored_by_default() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_unmatched_head()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn pr_with_unmatched_head_is_printed_if_requested() {
        // GIVEN
        let mut buffer = vec![];

        let behaviours = RunBehaviours::default().show_prs_with_unmatched_head();
        let mut l = RunLog::new(&mut buffer, &behaviours);
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_unmatched_head()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "improve tests" doesn't match the allowed head pattern ❌
"#
        );
    }

    #[test]
    fn pr_with_unknown_author_is_ignored_by_default() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_untrusted_author()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn pr_with_unknown_author_is_printed_if_requested() {
        // GIVEN
        let mut buffer = vec![];

        let behaviours = RunBehaviours::default().show_prs_from_untrusted_authors();
        let mut l = RunLog::new(&mut buffer, &behaviours);
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_unknown_author()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
        [ author ]   Github sent an empty user; skipping as I can't make any assumptions here ❌
"#
        );
    }

    #[test]
    fn pr_with_untrusted_author_is_ignored_by_default() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_untrusted_author()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn pr_with_untrusted_author_is_printed_if_requested() {
        // GIVEN
        let mut buffer = vec![];

        let behaviours = RunBehaviours::default().show_prs_from_untrusted_authors();
        let mut l = RunLog::new(&mut buffer, &behaviours);
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_untrusted_author()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
        [ author ]   "untrusted-dependabot[bot]" is not in the list of trusted authors ❌
"#
        );
    }

    #[test]
    fn pr_with_empty_check_conclusion_is_printed_correctly() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![
                merge_result_disqualified_check_with_unknown_conclusion(),
            ]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
        [ author ]   "dependabot[bot]" is in the list of trusted authors
        [ check  ]   "build (macos-latest)" concluded with desired status: "success"
        [ check  ]   "build (ubuntu-latest)" concluded with desired status: "success"
        [ check  ]   "test" concluded with desired status: "success"
        [ check  ]   Github returned with an empty conclusion for the check lint; skipping as I can't make any assumptions here ❌
"#
        );
    }

    #[test]
    fn pr_with_a_failed_check_is_printed_correctly() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_failed_check()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
        [ author ]   "dependabot[bot]" is in the list of trusted authors
        [ check  ]   "build (macos-latest)" concluded with desired status: "success"
        [ check  ]   "build (ubuntu-latest)" concluded with desired status: "success"
        [ check  ]   "test" concluded with desired status: "success"
        [ check  ]   "lint" concluded with undesired status: "failure" ❌
"#
        );
    }

    #[test]
    fn pr_with_unknown_state_is_printed_correctly() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_unknown_state()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
        [ author ]   "dependabot[bot]" is in the list of trusted authors
        [ check  ]   "build (macos-latest)" concluded with desired status: "success"
        [ check  ]   "build (ubuntu-latest)" concluded with desired status: "success"
        [ check  ]   "test" concluded with desired status: "success"
        [ state  ]   Github returned with an empty mergeable state; skipping as I can't make any assumptions here ❌
"#
        );
    }

    #[test]
    fn pr_with_an_undesirable_state_is_printed_correctly() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_disqualified_dirty_state()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
        [ author ]   "dependabot[bot]" is in the list of trusted authors
        [ check  ]   "build (macos-latest)" concluded with desired status: "success"
        [ check  ]   "build (ubuntu-latest)" concluded with desired status: "success"
        [ check  ]   "test" concluded with desired status: "success"
        [ state  ]   "dirty" is undesirable ❌
"#
        );
    }

    #[test]
    fn pr_with_a_finished_check_is_printed_correctly() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_qualified()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");
        assert_eq!(
            out,
            r#"

=============
  dhth/mrj
=============

-> checking PR #1
        build: bump clap from 4.5.39 to 4.5.40
        https://github.com/dhth/mrj/pull/1
        Created: Mon, 1 Jan 2024 01:01:01 +0000
        Updated: Tue, 2 Jan 2024 01:01:01 +0000
        [ head   ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
        [ author ]   "dependabot[bot]" is in the list of trusted authors
        [ check  ]   "build (macos-latest)" concluded with desired status: "success"
        [ check  ]   "build (ubuntu-latest)" concluded with desired status: "success"
        [ check  ]   "test" concluded with desired status: "success"
        [ state  ]   "clean" is desirable
        PR matches all criteria, I would've merged it if this weren't a dry run ✅
"#
        );
    }

    #[test]
    fn printing_summary_works() {
        // GIVEN
        let mut buffer = vec![];

        let mut l = RunLog::new(&mut buffer, &RunBehaviours::default());
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![
                merge_result_disqualified_unmatched_head(),
                merge_result_disqualified_unknown_author(),
                merge_result_disqualified_untrusted_author(),
                merge_result_disqualified_check_with_unknown_conclusion(),
                merge_result_disqualified_failed_check(),
                merge_result_disqualified_unknown_state(),
                merge_result_disqualified_dirty_state(),
                merge_result_qualified(),
            ]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);
        l.write_output().expect("output should've been written");

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");

        let (_, summary) = out
            .split_once(
                r#"
===========
  SUMMARY
===========
"#,
            )
            .expect("output should've been split by the summary header");

        assert_eq!(
            summary,
            r#"
# PRs merged                  :  0
# PRs disqualified            :  4
# Repos checked               :  1
# Repos with no relevant PRs  :  0
# Errors encountered          :  0
"#
        );
    }

    #[test]
    fn summary_includes_disqualifications_when_requested() {
        // GIVEN
        let mut buffer = vec![];

        let behaviours = RunBehaviours::default().summarize_disqualifications();

        let mut l = RunLog::new(&mut buffer, &behaviours);
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![
                merge_result_disqualified_unmatched_head(),
                merge_result_disqualified_unknown_author(),
                merge_result_disqualified_untrusted_author(),
                merge_result_disqualified_check_with_unknown_conclusion(),
                merge_result_disqualified_failed_check(),
                merge_result_disqualified_unknown_state(),
                merge_result_disqualified_dirty_state(),
                merge_result_qualified(),
            ]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);
        l.write_output().expect("output should've been written");

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");

        let (_, summary) = out
            .split_once(
                r#"
===========
  SUMMARY
===========
"#,
            )
            .expect("output should've been split by the summary header");

        assert_eq!(
            summary,
            r#"
# PRs merged                  :  0
# PRs disqualified            :  4
# Repos checked               :  1
# Repos with no relevant PRs  :  0
# Errors encountered          :  0

Disqualifications
---

- https://github.com/dhth/mrj/pull/1: check "lint" concluded with unknown status
- https://github.com/dhth/mrj/pull/1: check "lint" concluded with undesirable status "failure"
- https://github.com/dhth/mrj/pull/1: state is unknown
- https://github.com/dhth/mrj/pull/1: state "dirty" is undesirable
"#
        );
    }

    #[test]
    fn summary_includes_dq_that_are_ignored_by_default_if_requested() {
        // GIVEN
        let mut buffer = vec![];

        let behaviours = RunBehaviours::default()
            .show_prs_with_unmatched_head()
            .show_prs_from_untrusted_authors()
            .summarize_disqualifications();

        let mut l = RunLog::new(&mut buffer, &behaviours);
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![
                merge_result_disqualified_unmatched_head(),
                merge_result_disqualified_unknown_author(),
                merge_result_disqualified_untrusted_author(),
                merge_result_disqualified_check_with_unknown_conclusion(),
                merge_result_disqualified_failed_check(),
                merge_result_disqualified_unknown_state(),
                merge_result_disqualified_dirty_state(),
                merge_result_qualified(),
            ]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);
        l.write_output().expect("output should've been written");

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");

        let (_, summary) = out
            .split_once(
                r#"
===========
  SUMMARY
===========
"#,
            )
            .expect("output should've been split by the summary header");

        assert_eq!(
            summary,
            r#"
# PRs merged                  :  0
# PRs disqualified            :  7
# Repos checked               :  1
# Repos with no relevant PRs  :  0
# Errors encountered          :  0

Disqualifications
---

- https://github.com/dhth/mrj/pull/1: head didn't match
- https://github.com/dhth/mrj/pull/1: github returned empty author
- https://github.com/dhth/mrj/pull/1: author "untrusted-dependabot[bot]" is not in the list of trusted authors
- https://github.com/dhth/mrj/pull/1: check "lint" concluded with unknown status
- https://github.com/dhth/mrj/pull/1: check "lint" concluded with undesirable status "failure"
- https://github.com/dhth/mrj/pull/1: state is unknown
- https://github.com/dhth/mrj/pull/1: state "dirty" is undesirable
"#
        );
    }

    #[test]
    fn summary_doesnt_include_dq_if_none_exist() {
        // GIVEN
        let mut buffer = vec![];

        let behaviours = RunBehaviours::default().summarize_disqualifications();

        let mut l = RunLog::new(&mut buffer, &behaviours);
        let repo_check = RepoCheck {
            owner: OWNER.to_string(),
            name: REPO.to_string(),
            state: RepoCheckFinished(vec![merge_result_qualified()]),
        };
        let repo_result = RepoResult::Finished(repo_check);

        // WHEN
        l.add_repo_result(repo_result);
        l.write_output().expect("output should've been written");

        // THEN
        let out = String::from_utf8(buffer)
            .expect("buffer contents should've been converted to a string");

        let (_, summary) = out
            .split_once(
                r#"
===========
  SUMMARY
===========
"#,
            )
            .expect("output should've been split by the summary header");

        assert_eq!(
            summary,
            r#"
# PRs merged                  :  0
# PRs disqualified            :  0
# Repos checked               :  1
# Repos with no relevant PRs  :  0
# Errors encountered          :  0
"#
        );
    }

    fn merge_result_disqualified_unmatched_head() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![],
            state: PRDisqualified(Disqualification::Head("improve tests".to_string())),
        })
    }

    fn merge_result_disqualified_unknown_author() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::Author(None)),
        })
    }

    fn merge_result_disqualified_untrusted_author() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![Qualification::Head(PR_HEAD.to_string())],
            state: PRDisqualified(Disqualification::Author(Some(
                "untrusted-dependabot[bot]".to_string(),
            ))),
        })
    }

    fn merge_result_disqualified_check_with_unknown_conclusion() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![
                Qualification::Head(PR_HEAD.to_string()),
                Qualification::Author(PR_AUTHOR.to_string()),
                Qualification::Check {
                    name: "build (macos-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "build (ubuntu-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "test".to_string(),
                    conclusion: "success".to_string(),
                },
            ],
            state: PRDisqualified(Disqualification::Check {
                name: "lint".to_string(),
                conclusion: None,
            }),
        })
    }

    fn merge_result_disqualified_failed_check() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![
                Qualification::Head(PR_HEAD.to_string()),
                Qualification::Author(PR_AUTHOR.to_string()),
                Qualification::Check {
                    name: "build (macos-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "build (ubuntu-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "test".to_string(),
                    conclusion: "success".to_string(),
                },
            ],
            state: PRDisqualified(Disqualification::Check {
                name: "lint".to_string(),
                conclusion: Some("failure".to_string()),
            }),
        })
    }

    fn merge_result_disqualified_unknown_state() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![
                Qualification::Head(PR_HEAD.to_string()),
                Qualification::Author(PR_AUTHOR.to_string()),
                Qualification::Check {
                    name: "build (macos-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "build (ubuntu-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "test".to_string(),
                    conclusion: "success".to_string(),
                },
            ],
            state: PRDisqualified(Disqualification::State(None)),
        })
    }

    fn merge_result_disqualified_dirty_state() -> MergeResult {
        MergeResult::Disqualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![
                Qualification::Head(PR_HEAD.to_string()),
                Qualification::Author(PR_AUTHOR.to_string()),
                Qualification::Check {
                    name: "build (macos-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "build (ubuntu-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "test".to_string(),
                    conclusion: "success".to_string(),
                },
            ],
            state: PRDisqualified(Disqualification::State(Some("dirty".to_string()))),
        })
    }

    fn merge_result_qualified() -> MergeResult {
        MergeResult::Qualified(PRCheck {
            number: 1,
            title: PR_TITLE.to_string(),
            url: PR_URL.to_string(),
            pr_created_at: Some(created_at()),
            pr_updated_at: Some(updated_at()),
            qualifications: vec![
                Qualification::Head(PR_HEAD.to_string()),
                Qualification::Author(PR_AUTHOR.to_string()),
                Qualification::Check {
                    name: "build (macos-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "build (ubuntu-latest)".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::Check {
                    name: "test".to_string(),
                    conclusion: "success".to_string(),
                },
                Qualification::State("clean".to_string()),
            ],
            state: PRCheckFinished,
        })
    }

    fn created_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 1, 1, 1, 1).unwrap()
    }

    fn updated_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 2, 1, 1, 1).unwrap()
    }
}
