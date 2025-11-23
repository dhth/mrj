use anyhow::Context;
use chrono::Utc;
use regex::Regex;
use std::path::PathBuf;

use crate::domain::ReportConfig;

const MRJ_DIR: &str = ".mrj";
const RUNS_DIR: &str = "runs";
const DIST_DIR: &str = "dist";
const RUN_NUMBER_FILE: &str = "last-run.txt";

pub fn generate_report(config: &ReportConfig) -> anyhow::Result<()> {
    // FILE MANAGEMENT
    let mrj_dir = PathBuf::from(MRJ_DIR);
    let runs_dir = mrj_dir.join(RUNS_DIR);
    if !runs_dir.exists() {
        std::fs::create_dir_all(&runs_dir).context("couldn't create runs directory")?;
    }
    let dist_dir = PathBuf::from(DIST_DIR);
    let run_number_file_path = mrj_dir.join(RUN_NUMBER_FILE);
    let last_run_number = super::io::get_last_run_number(&run_number_file_path)
        .context("couldn't get last run number")?;
    let run_number = last_run_number + 1;
    let now = Utc::now();
    let date = now.format("%a-%b-%d").to_string().to_lowercase();
    let new_run_file = runs_dir.join(format!("run-{run_number}--{date}.txt"));

    std::fs::copy(&config.output_path, new_run_file)
        .context("couldn't copy latest run to mrj's \"runs\" directory")?;

    #[allow(clippy::expect_used)]
    let file_regex =
        Regex::new(r"^run-(\d+)[^\.]*\.txt$").expect("regex for run files should've been built");

    super::io::keep_last_n_outputs(&runs_dir, config.num_runs, &file_regex)?;

    if dist_dir.is_dir()
        && dist_dir
            .try_exists()
            .context("couldn't check if \"dist\" dir exists")?
    {
        std::fs::remove_dir_all(&dist_dir).context("couldn't delete the existing \"dist\" dir")?;
    }

    // CREATE REPORT
    std::fs::create_dir(&dist_dir).context("couldn't create \"dist\" dir")?;

    let run_data = super::io::gather_run_data(runs_dir, &file_regex)
        .context("couldn't gather data from previous runs")?;

    let report_contents = super::html::render_report(
        run_data.as_slice(),
        now,
        config.custom_template.as_deref(),
        &config.title,
    )
    .context("couldn't render report")?;

    // WRITE AND OPEN
    super::io::write_report(&report_contents, &dist_dir).context("couldn't write report")?;

    super::io::update_run_number(run_number, &run_number_file_path).with_context(|| {
        format!(
            "couldn't update run number in {}",
            run_number_file_path.to_string_lossy()
        )
    })?;

    if config.open_report {
        let index_path = dist_dir.join("index.html");
        if open::that(index_path).is_err() {
            eprintln!(
                "couldn't open report in your browser, report is available in the \"{DIST_DIR}\" directory"
            );
        }
    } else {
        println!("report is available in the \"{DIST_DIR}\" directory 🚀");
    }

    Ok(())
}
