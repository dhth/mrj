use crate::domain::{Disqualification, Qualification, RunStats};
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
}

impl RunLog {
    pub(super) fn new(output: bool) -> Self {
        RunLog {
            write_to_file: output,
            lines: vec![],
            stats: RunStats::default(),
        }
    }

    pub fn banner(&mut self, dry_run: bool) {
        println!("{}", BANNER.green().bold());
        if dry_run {
            println!("{}", "                         dry run".yellow());
        }
        println!("\n");

        if self.write_to_file {
            self.lines.push(BANNER.to_string());
            if dry_run {
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

    pub fn disqualification(&mut self, disqualification: Disqualification) {
        let msg = match disqualification {
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

    pub fn merge(&mut self, msg: &str, dry_run: bool) {
        println!("        {}", msg.green());

        if self.write_to_file {
            self.lines.push(format!("        {}", msg));
        }

        if !dry_run {
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
