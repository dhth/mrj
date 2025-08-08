use anyhow::Context;
use chrono::Utc;
use regex::Regex;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MRJ_DIR: &str = ".mrj";
const RUNS_DIR: &str = "runs";
const DIST_DIR: &str = "dist";
const RUN_NUMBER_FILE: &str = "last-run.txt";
const RUN_TEMPLATE: &str = include_str!("./assets/templates/run.html");
const INDEX_TEMPLATE: &str = include_str!("./assets/templates/index.html");
const FAVICON_BYTES: &[u8] = include_bytes!("./assets/static/favicon.png");
const BUILD_NUM_PLACEHOLDER: &str = "{{BUILD_NUM}}";
const CONTENT_PLACEHOLDER: &str = "{{CONTENT}}";
const RUN_LIST_PLACEHOLDER: &str = "{{RUN_LIST}}";

pub fn generate_report<P>(output_file: P, open_report: bool, num_runs: u8) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mrj_dir = PathBuf::from(MRJ_DIR);
    let runs_dir = mrj_dir.join(RUNS_DIR);
    if !runs_dir.exists() {
        fs::create_dir_all(&runs_dir).context("couldn't create runs directory")?;
    }
    let dist_dir = PathBuf::from(DIST_DIR);
    let run_number_file_path = mrj_dir.join(RUN_NUMBER_FILE);
    let last_run_number =
        get_last_run_number(&run_number_file_path).context("couldn't get last run number")?;
    let run_number = last_run_number + 1;
    let now = Utc::now();
    let date = now.format("%a-%b-%d").to_string().to_lowercase();
    let new_run_file = runs_dir.join(format!("run-{run_number}--{date}.txt"));

    fs::copy(output_file, new_run_file)
        .context("couldn't copy latest run to mrj's \"runs\" directory")?;

    keep_last_n_outputs(&runs_dir, num_runs)?;

    if dist_dir.is_dir()
        && dist_dir
            .try_exists()
            .context("couldn't check if \"dist\" dir exists")?
    {
        fs::remove_dir_all(&dist_dir).context("couldn't delete the existing \"dist\" dir")?;
    }

    fs::create_dir(&dist_dir).context("couldn't create \"dist\" dir")?;

    generate_run_html_outputs(&runs_dir, &dist_dir)
        .context("couldn't generate output HTML files")?;
    update_run_number(run_number, &run_number_file_path).with_context(|| {
        format!(
            "couldn't update run number in {}",
            run_number_file_path.to_string_lossy()
        )
    })?;
    generate_index_output(&runs_dir, &dist_dir)
        .context("couldn't generate index page of the report")?;

    let _ = write_favicon(&dist_dir);

    if open_report {
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

fn get_last_run_number<P>(path: P) -> anyhow::Result<u16>
where
    P: AsRef<Path>,
{
    if !path.as_ref().exists() {
        return Ok(0);
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let run_number = contents
        .trim()
        .parse::<u16>()
        .context("run number file doesn't contain a valid number")?;

    Ok(run_number)
}

fn keep_last_n_outputs<P>(dir: P, n: u8) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    #[allow(clippy::unwrap_used)]
    let re = Regex::new(r"^run-(\d+)[^\.]*\.txt$").unwrap();

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|res| res.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }

            let file_name = e.file_name();
            let file_name_str = file_name.to_string_lossy();
            let caps = re.captures(&file_name_str)?;
            let num: u64 = caps.get(1)?.as_str().parse().ok()?;
            Some((e.path(), num))
        })
        .collect();

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    for (path, _) in entries.into_iter().skip(n as usize) {
        println!("[INFO] deleting older run file: {}", path.to_string_lossy());
        if let Err(err) = fs::remove_file(&path) {
            eprintln!(
                "couldn't delete older run file: {}, you might want to delete it manually, error: {}",
                &path.to_string_lossy(),
                err
            );
        }
    }

    Ok(())
}

fn generate_run_html_outputs<P>(runs_dir: P, dist_dir: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    #[allow(clippy::unwrap_used)]
    let re = Regex::new(r"^run-(\d+)[^\.]*\.txt$").unwrap();
    for entry in fs::read_dir(runs_dir.as_ref())? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if let Some(caps) = re.captures(&file_name)
            && let Some(num_match) = caps.get(1)
        {
            let num: u16 = num_match.as_str().parse()?;
            let mut contents = String::new();
            File::open(entry.path())?.read_to_string(&mut contents)?;
            generate_run_html(&file_name, num, &contents, dist_dir.as_ref())?;
        }
    }
    Ok(())
}

fn generate_run_html<P>(
    file_name: &str,
    run_number: u16,
    contents: &str,
    dist_dir: P,
) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let run_file_contents = RUN_TEMPLATE
        .replace(BUILD_NUM_PLACEHOLDER, &run_number.to_string())
        .replace(CONTENT_PLACEHOLDER, contents);

    let output_path = dist_dir.as_ref().join(file_name.replace(".txt", ".html"));

    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_path)?;

    output_file.write_all(run_file_contents.as_bytes())?;

    Ok(())
}
fn update_run_number<P>(run_number: u16, path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    file.write_all(format!("{run_number}").as_bytes())?;

    Ok(())
}

fn generate_index_output<P>(runs_dir: P, dist_dir: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    #[allow(clippy::unwrap_used)]
    let re = Regex::new(r"^run-(\d+)[^\.]*\.txt$").unwrap();

    let mut entries: Vec<_> = fs::read_dir(runs_dir)?
        .filter_map(|res| res.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }

            let file_name = e.file_name();
            let file_name_str = file_name.to_string_lossy();
            let caps = re.captures(&file_name_str)?;
            let num: u64 = caps.get(1)?.as_str().parse().ok()?;
            Some((e.path(), num))
        })
        .collect();

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let list_elements = entries
        .iter()
        .filter_map(|(p, _)| {
            let path_str = match p.file_name() {
                Some(path) => path.to_string_lossy(),
                None => return None,
            };

            let stem = path_str.replace(".txt", "");
            let link_text = match stem.split_once("--") {
                Some((r, d)) => format!("{r} ({d})"),
                None => stem.to_string(),
            };

            Some(format!(
                r#"<li><a class="hover:underline" href="{}">{}</a></li>"#,
                path_str.replace(".txt", ".html"),
                link_text,
            ))
        })
        .collect::<Vec<String>>();

    let run_file_contents = INDEX_TEMPLATE.replace(RUN_LIST_PLACEHOLDER, &list_elements.join("\n"));

    let output_file_path = dist_dir.as_ref().join("index.html");
    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file_path)?;

    output_file.write_all(run_file_contents.as_bytes())?;

    Ok(())
}

fn write_favicon<P: AsRef<Path>>(dist_dir: P) -> anyhow::Result<()> {
    let static_dir = dist_dir.as_ref().join("static");
    fs::create_dir(&static_dir)?;

    let favicon_path = &static_dir.join("favicon.png");

    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(favicon_path)?;

    output_file.write_all(FAVICON_BYTES)?;

    Ok(())
}
