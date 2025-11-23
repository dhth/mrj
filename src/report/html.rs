use super::data::RunData;
use anyhow::Context;
use chrono::{DateTime, Utc};
use tera::{Context as TeraContext, Tera};

const BUILTIN_TEMPLATE: &str = include_str!("./assets/templates/index.html");
const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

pub(super) fn render_report(
    runs: &[RunData],
    reference_time: DateTime<Utc>,
    custom_template: Option<&str>,
    title: &str,
) -> anyhow::Result<String> {
    let mut tera = Tera::default();
    match custom_template {
        Some(template) => tera
            .add_raw_template("template.html", template)
            .context("failed to parse HTML template")?,
        None => tera
            .add_raw_template("template.html", BUILTIN_TEMPLATE)
            .context("failed to parse built-in HTML template")?,
    }

    let mut tera_ctx = TeraContext::new();
    tera_ctx.insert("title", title);
    tera_ctx.insert(
        "timestamp",
        &reference_time.format(TIMESTAMP_FORMAT).to_string(),
    );
    tera_ctx.insert("runs", runs);

    let page_contents = tera
        .render("template.html", &tera_ctx)
        .context("failed to render HTML template")?;

    Ok(page_contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use insta::assert_snapshot;

    const CONTENT1: &str = include_str!("testdata/rundata/runs/run-576--sat-nov-01.txt");
    const CONTENT2: &str = include_str!("testdata/rundata/runs/run-577--sun-nov-02.txt");
    const CUSTOM_TEMPLATE: &str = include_str!("testdata/custom_template.html");

    #[test]
    fn render_report_works_for_builtin_template() -> anyhow::Result<()> {
        // GIVEN
        let runs = vec![
            RunData {
                label: "run-576 (sat-nov-01)".into(),
                contents: CONTENT1.trim().to_string(),
            },
            RunData {
                label: "run-577 (sun-nov-02)".into(),
                contents: CONTENT2.trim().to_string(),
            },
        ];
        let now = Utc.with_ymd_and_hms(2025, 1, 16, 12, 0, 0).unwrap();

        // WHEN
        let result = render_report(runs.as_slice(), now, None, "mrj runs")?;

        // THEN
        assert_snapshot!(result);

        Ok(())
    }

    #[test]
    fn render_report_works_for_custom_template() -> anyhow::Result<()> {
        // GIVEN
        let runs = vec![
            RunData {
                label: "run-576 (sat-nov-01)".into(),
                contents: CONTENT1.trim().to_string(),
            },
            RunData {
                label: "run-577 (sun-nov-02)".into(),
                contents: CONTENT2.trim().to_string(),
            },
        ];
        let now = Utc.with_ymd_and_hms(2025, 1, 16, 12, 0, 0).unwrap();

        // WHEN
        let result = render_report(
            runs.as_slice(),
            now,
            Some(CUSTOM_TEMPLATE),
            "custom template",
        )?;

        // THEN
        assert_snapshot!(result);

        Ok(())
    }
}
