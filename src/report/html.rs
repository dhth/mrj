use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tera::{Context as TeraContext, Tera};

use crate::persistence::schema::StoredRunData;

const BUILTIN_TEMPLATE: &str = include_str!("./assets/templates/index.html");

#[derive(Serialize)]
struct ReportContext<'a> {
    title: &'a str,
    timestamp: DateTime<Utc>,
    runs: &'a [StoredRunData],
}

pub(super) fn render_report(
    runs: &[StoredRunData],
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

    let tera_ctx = TeraContext::from_serialize(ReportContext {
        title,
        timestamp: reference_time,
        runs,
    })
    .context("failed to build report context")?;

    let page_contents = tera
        .render("template.html", &tera_ctx)
        .context("failed to render HTML template")?;

    Ok(page_contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::schema::{
        StoredDisqualification, StoredMergeType, StoredPrRecord, StoredPrStatus,
        StoredQualification, StoredRepoRecord, StoredRepoStatus, StoredRunConfig, StoredRunData,
        StoredRunFlags, StoredRunMode, StoredRunSummary, StoredSortBy, StoredSortDirection,
    };
    use chrono::TimeZone;
    use insta::assert_snapshot;

    const CUSTOM_TEMPLATE: &str = include_str!("testdata/custom_template.html");

    #[test]
    fn render_report_works_for_builtin_template() -> anyhow::Result<()> {
        // GIVEN
        let runs = sample_runs();
        let now = Utc
            .with_ymd_and_hms(2025, 1, 16, 12, 0, 0)
            .single()
            .unwrap();

        // WHEN
        let result = render_report(runs.as_slice(), now, None, "mrj runs")?;

        // THEN
        assert_snapshot!(result);

        Ok(())
    }

    #[test]
    fn render_report_works_for_custom_template() -> anyhow::Result<()> {
        // GIVEN
        let runs = sample_runs();
        let now = Utc
            .with_ymd_and_hms(2025, 1, 16, 12, 0, 0)
            .single()
            .unwrap();

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

    fn sample_runs() -> Vec<StoredRunData> {
        vec![
            StoredRunData {
                started_at: Utc
                    .with_ymd_and_hms(2025, 11, 2, 23, 31, 11)
                    .single()
                    .unwrap(),
                finished_at: Utc
                    .with_ymd_and_hms(2025, 11, 2, 23, 31, 47)
                    .single()
                    .unwrap(),
                took_ms: 36_000,
                mode: StoredRunMode::DryRun,
                config: StoredRunConfig {
                    base_branch: Some("main".into()),
                    head_pattern: Some("(dependabot|update)".into()),
                    merge_if_blocked: false,
                    merge_if_checks_skipped: true,
                    merge_if_checks_neutral: false,
                    merge_type: StoredMergeType::Squash,
                    sort_by: StoredSortBy::Created,
                    sort_direction: StoredSortDirection::Asc,
                    flags: StoredRunFlags {
                        show_repos_with_no_prs: false,
                        show_prs_from_untrusted_authors: false,
                        show_prs_with_unmatched_head: false,
                        skip_disqualifications_in_summary: false,
                    },
                },
                summary: StoredRunSummary {
                    num_disqualifications: 1,
                    num_errors: 2,
                    num_merged: 0,
                },
                repos: vec![
                    StoredRepoRecord {
                        repo: "dhth/mrj".into(),
                        owner: "dhth".into(),
                        name: "mrj".into(),
                        status: StoredRepoStatus::Finished,
                        error: None,
                        prs: vec![
                            StoredPrRecord {
                                number: 12,
                                title: "build: bump clap from 4.5.39 to 4.5.40".into(),
                                url: "https://github.com/dhth/mrj/pull/12".into(),
                                created_at: Some(
                                    Utc.with_ymd_and_hms(2025, 11, 2, 23, 0, 0)
                                        .single()
                                        .unwrap(),
                                ),
                                updated_at: Some(
                                    Utc.with_ymd_and_hms(2025, 11, 2, 23, 10, 0)
                                        .single()
                                        .unwrap(),
                                ),
                                status: StoredPrStatus::Disqualified,
                                qualifications: vec![StoredQualification::Head {
                                    value: "dependabot/cargo/clap-4.5.40".into(),
                                }],
                                disqualification: Some(StoredDisqualification::Author {
                                    value: Some("untrusted-bot".into()),
                                }),
                                error: None,
                                merged: false,
                            },
                            StoredPrRecord {
                                number: 13,
                                title: "build: bump tera from 1.19.0 to 1.20.1".into(),
                                url: "https://github.com/dhth/mrj/pull/13".into(),
                                created_at: Some(
                                    Utc.with_ymd_and_hms(2025, 11, 2, 23, 5, 0)
                                        .single()
                                        .unwrap(),
                                ),
                                updated_at: Some(
                                    Utc.with_ymd_and_hms(2025, 11, 2, 23, 12, 0)
                                        .single()
                                        .unwrap(),
                                ),
                                status: StoredPrStatus::Qualified,
                                qualifications: vec![
                                    StoredQualification::Head {
                                        value: "dependabot/cargo/tera-1.20.1".into(),
                                    },
                                    StoredQualification::Author {
                                        value: "dependabot[bot]".into(),
                                    },
                                    StoredQualification::Check {
                                        name: "test".into(),
                                        conclusion: "success".into(),
                                    },
                                ],
                                disqualification: None,
                                error: None,
                                merged: false,
                            },
                            StoredPrRecord {
                                number: 14,
                                title: "build: bump regex from 1.12.2 to 1.12.3".into(),
                                url: "https://github.com/dhth/mrj/pull/14".into(),
                                created_at: Some(
                                    Utc.with_ymd_and_hms(2025, 11, 2, 23, 8, 0)
                                        .single()
                                        .unwrap(),
                                ),
                                updated_at: Some(
                                    Utc.with_ymd_and_hms(2025, 11, 2, 23, 15, 0)
                                        .single()
                                        .unwrap(),
                                ),
                                status: StoredPrStatus::Errored,
                                qualifications: vec![StoredQualification::Head {
                                    value: "dependabot/cargo/regex-1.12.3".into(),
                                }],
                                disqualification: None,
                                error: Some("GitHub API returned a transient error".into()),
                                merged: false,
                            },
                        ],
                    },
                    StoredRepoRecord {
                        repo: "dhth/bmm".into(),
                        owner: "dhth".into(),
                        name: "bmm".into(),
                        status: StoredRepoStatus::Errored,
                        error: Some("couldn't fetch open PRs for repo".into()),
                        prs: vec![],
                    },
                ],
            },
            StoredRunData {
                started_at: Utc
                    .with_ymd_and_hms(2025, 11, 1, 22, 32, 41)
                    .single()
                    .unwrap(),
                finished_at: Utc
                    .with_ymd_and_hms(2025, 11, 1, 22, 33, 11)
                    .single()
                    .unwrap(),
                took_ms: 30_000,
                mode: StoredRunMode::Execute,
                config: StoredRunConfig {
                    base_branch: Some("main".into()),
                    head_pattern: Some("dependabot".into()),
                    merge_if_blocked: false,
                    merge_if_checks_skipped: false,
                    merge_if_checks_neutral: true,
                    merge_type: StoredMergeType::Squash,
                    sort_by: StoredSortBy::Updated,
                    sort_direction: StoredSortDirection::Desc,
                    flags: StoredRunFlags {
                        show_repos_with_no_prs: true,
                        show_prs_from_untrusted_authors: false,
                        show_prs_with_unmatched_head: false,
                        skip_disqualifications_in_summary: false,
                    },
                },
                summary: StoredRunSummary {
                    num_disqualifications: 0,
                    num_errors: 0,
                    num_merged: 1,
                },
                repos: vec![StoredRepoRecord {
                    repo: "dhth/mrj".into(),
                    owner: "dhth".into(),
                    name: "mrj".into(),
                    status: StoredRepoStatus::Finished,
                    error: None,
                    prs: vec![StoredPrRecord {
                        number: 11,
                        title: "build: bump octocrab from 0.49.6 to 0.49.7".into(),
                        url: "https://github.com/dhth/mrj/pull/11".into(),
                        created_at: Some(
                            Utc.with_ymd_and_hms(2025, 11, 1, 22, 20, 0)
                                .single()
                                .unwrap(),
                        ),
                        updated_at: Some(
                            Utc.with_ymd_and_hms(2025, 11, 1, 22, 28, 0)
                                .single()
                                .unwrap(),
                        ),
                        status: StoredPrStatus::Qualified,
                        qualifications: vec![
                            StoredQualification::Head {
                                value: "dependabot/cargo/octocrab-0.49.7".into(),
                            },
                            StoredQualification::Author {
                                value: "dependabot[bot]".into(),
                            },
                            StoredQualification::Check {
                                name: "advisory".into(),
                                conclusion: "neutral".into(),
                            },
                        ],
                        disqualification: None,
                        error: None,
                        merged: true,
                    }],
                }],
            },
        ]
    }
}
