use crate::domain::{
    Disqualification, MergeFailure, PRResult, Qualification, RepoResult, RunStats,
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
    stats: RunStats,
    dry_run: bool,
}

impl RunLog {
    pub(super) fn new(output: bool, dry_run: bool) -> Self {
        RunLog {
            write_to_file: output,
            lines: vec![],
            stats: RunStats::default(),
            dry_run,
        }
    }

    pub fn add_result(&mut self, result: RepoResult) {
        self.repo_info(&result.name());

        match result.result {
            Ok(pr_results) if pr_results.is_empty() => {
                self.empty_line();
                self.absence("no PRs");
            }
            Ok(pr_results) => pr_results.into_iter().for_each(|r| self.add_pr_result(r)),
            Err(err) => self.error(err),
        }
    }

    fn add_pr_result(&mut self, result: PRResult) {
        self.pr_info(&format!(
            r#"
-> checking PR #{}
        {}
        {}"#,
            result.number, result.title, result.url,
        ));

        for q in result.qualifications {
            self.qualification(q);
        }

        match result.failure {
            Some(failure) => match failure {
                MergeFailure::Disqualification(dq) => self.disqualification(dq),
                MergeFailure::UnexpectedError(err) => self.error(err),
            },
            None => self.merge(),
        }
    }

    pub fn banner(&mut self) {
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

    pub fn info(&mut self, message: &str) {
        println!("[INFO] {}", message);

        if self.write_to_file {
            self.lines.push(format!("[INFO] {}", message));
        }
    }

    pub fn repo_info(&mut self, name: &str) {
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

    pub fn pr_info(&mut self, msg: &str) {
        println!("{}", msg.purple());

        if self.write_to_file {
            self.lines.push(msg.to_string());
        }
    }

    pub fn qualification(&mut self, q: Qualification) {
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
            Qualification::State(s) => format!("{} \"{}\" looks good", STATE, s),
        };

        println!("        {}", msg.blue());

        if self.write_to_file {
            self.lines.push(format!("        {}", msg));
        }
    }

    pub fn disqualification(&mut self, dq: Disqualification) {
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

        self.stats.record_disqualification();
    }

    pub fn absence(&mut self, msg: &str) {
        println!("        {}", msg.yellow());

        if self.write_to_file {
            self.lines.push(format!("        {}", msg));
        }
    }

    pub fn merge(&mut self) {
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
            self.stats.record_merge();
        }
    }

    pub fn empty_line(&mut self) {
        println!();

        if self.write_to_file {
            self.lines.push("".to_string());
        }
    }

    pub fn error(&mut self, error: anyhow::Error) {
        println!("{}", format!("        error 😵: {}", error).red());

        if self.write_to_file {
            self.lines.push(format!("        error 😵: {}", error));
        }

        self.stats.record_error();
    }

    pub fn write_output<P>(&mut self, write_to_file: bool, output_file: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let stats = format!(
            r#"
===========================

  Stats

  #PRs merged         : {}
  #PRs disqualified   : {}
  #errors encountered : {}

==========================="#,
            self.stats.num_merges, self.stats.num_disqualifications, self.stats.num_errors
        );

        println!("{}", &stats.green());

        if !write_to_file {
            return Ok(());
        }

        self.lines.push(stats);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_file)
            .context("couldn't open a handle to the output file")?;

        file.write_all(self.lines.join("\n").as_bytes())
            .context("couldn't write output to file")?;

        Ok(())
    }
}
