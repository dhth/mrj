use super::data::RunData;
use anyhow::Context;
use regex::Regex;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub(super) fn get_last_run_number<P>(path: P) -> anyhow::Result<u16>
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

pub(super) fn keep_last_n_outputs<P>(dir: P, n: u8, file_regex: &Regex) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|res| res.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }

            let file_name = e.file_name();
            let file_name_str = file_name.to_string_lossy();
            let caps = file_regex.captures(&file_name_str)?;
            let num: u64 = caps.get(1)?.as_str().parse().ok()?;
            Some((e.path(), num))
        })
        .collect();

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    for (path, _) in entries.into_iter().skip(n as usize) {
        println!("[INFO] deleting older run file: {}", path.to_string_lossy());
        if let Err(err) = std::fs::remove_file(&path) {
            eprintln!(
                "couldn't delete older run file: {}, you might want to delete it manually, error: {}",
                &path.to_string_lossy(),
                err
            );
        }
    }

    Ok(())
}

pub(super) fn update_run_number<P>(run_number: u16, path: P) -> anyhow::Result<()>
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

pub(super) fn gather_run_data<P>(runs_dir: P, file_regex: &Regex) -> anyhow::Result<Vec<RunData>>
where
    P: AsRef<Path>,
{
    let mut entries: Vec<_> = std::fs::read_dir(runs_dir)?
        .filter_map(|res| res.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }

            let file_name = e.file_name();
            let file_name_str = file_name.to_string_lossy();
            let caps = file_regex.captures(&file_name_str)?;
            let num: u64 = caps.get(1)?.as_str().parse().ok()?;
            Some((e.path(), num))
        })
        .collect();

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let mut runs = Vec::new();

    for (path, _) in entries.iter() {
        let file_name_os = match path.file_name() {
            Some(name) => name,
            None => continue,
        };

        let file_name = file_name_os.to_string_lossy();
        let stem = file_name.replace(".txt", "");
        let (run_id, date_part) = match stem.split_once("--") {
            Some((r, d)) => (r.to_string(), Some(d.to_string())),
            None => (stem.to_string(), None),
        };

        let label = match date_part {
            Some(d) => format!("{run_id} ({d})"),
            None => run_id,
        };

        let mut contents = String::new();
        File::open(path)?.read_to_string(&mut contents)?;
        contents = contents.trim_end().to_string();

        runs.push(RunData { label, contents });
    }

    Ok(runs)
}

pub(super) fn write_report<S, P>(contents: S, dist_dir: P) -> anyhow::Result<()>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let output_file_path = dist_dir.as_ref().join("index.html");
    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file_path)?;

    output_file.write_all(contents.as_ref().as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_yaml_snapshot;

    #[test]
    fn gather_run_data_works_correctly() -> anyhow::Result<()> {
        // GIVEN
        let runs_dir = "src/report/testdata/rundata/runs";
        let file_regex = Regex::new(r"^run-(\d+)[^\.]*\.txt$")?;

        // WHEN
        let result = gather_run_data(runs_dir, &file_regex)?;

        // THEN
        assert_yaml_snapshot!(result);

        Ok(())
    }
}
