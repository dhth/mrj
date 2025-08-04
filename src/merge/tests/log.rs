use super::super::{RunBehaviours, RunLog};
use crate::domain::{Disqualification, MergeResult, Qualification, RepoResult};
use crate::domain::{
    PRCheck, PRCheckFinished, PRDisqualified, RepoCheck, RepoCheckErrored, RepoCheckFinished,
};
use chrono::{DateTime, TimeZone, Utc};
use insta::assert_snapshot;

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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"

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
        state: RepoCheckFinished(vec![merge_result_disqualified_unmatched_head(1)]),
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
        state: RepoCheckFinished(vec![merge_result_disqualified_unmatched_head(1)]),
    };
    let repo_result = RepoResult::Finished(repo_check);

    // WHEN
    l.add_repo_result(repo_result);

    // THEN
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "improve tests" doesn't match the allowed head pattern ❌
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");
    assert_snapshot!(
        out,
        @r#"
        =============
          dhth/mrj
        =============

        -> checking PR #1
                build: bump clap from 4.5.39 to 4.5.40
                https://github.com/dhth/mrj/pull/1
                Created: Mon, 1 Jan 2024 01:01:01 +0000
                Updated: Tue, 2 Jan 2024 01:01:01 +0000
                [ head  ]   "dependabot/cargo/clap-4.5.40" matches the allowed head pattern
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
            merge_result_disqualified_unmatched_head(1),
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");

    let (_, summary) = out
        .split_once(
            r#"
===========
  SUMMARY
===========
"#,
        )
        .expect("output should've been split by the summary header");

    assert_snapshot!(
        summary,
        @r"
        - PRs merged:                    0
        - PRs disqualified:              4
        - Repos checked:                 1
        - Repos with no relevant PRs:    0
        - Errors encountered:            0

        Disqualifications
        ---

        - https://github.com/dhth/mrj/pull/1        check lint: unknown conclusion
        - https://github.com/dhth/mrj/pull/1        check lint: failure
        - https://github.com/dhth/mrj/pull/1        state: unknown
        - https://github.com/dhth/mrj/pull/1        state: dirty
        "
    );
}

#[test]
fn disqualifications_can_be_skipped_in_summary_when_requested() {
    // GIVEN
    let mut buffer = vec![];

    let behaviours = RunBehaviours::default().skip_disqualifications_in_summary();

    let mut l = RunLog::new(&mut buffer, &behaviours);
    let repo_check = RepoCheck {
        owner: OWNER.to_string(),
        name: REPO.to_string(),
        state: RepoCheckFinished(vec![
            merge_result_disqualified_unmatched_head(1),
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");

    let (_, summary) = out
        .split_once(
            r#"
===========
  SUMMARY
===========
"#,
        )
        .expect("output should've been split by the summary header");

    assert_snapshot!(
        summary,
        @r"
        - PRs merged:                    0
        - PRs disqualified:              4
        - Repos checked:                 1
        - Repos with no relevant PRs:    0
        - Errors encountered:            0
        "
    );
}

#[test]
fn summary_includes_dq_that_are_ignored_by_default_if_requested() {
    // GIVEN
    let mut buffer = vec![];

    let behaviours = RunBehaviours::default()
        .show_prs_with_unmatched_head()
        .show_prs_from_untrusted_authors();

    let mut l = RunLog::new(&mut buffer, &behaviours);
    let repo_check = RepoCheck {
        owner: OWNER.to_string(),
        name: REPO.to_string(),
        state: RepoCheckFinished(vec![
            merge_result_disqualified_unmatched_head(1),
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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");

    let (_, summary) = out
        .split_once(
            r#"
===========
  SUMMARY
===========
"#,
        )
        .expect("output should've been split by the summary header");

    assert_snapshot!(
        summary,
        @r"
        - PRs merged:                    0
        - PRs disqualified:              7
        - Repos checked:                 1
        - Repos with no relevant PRs:    0
        - Errors encountered:            0

        Disqualifications
        ---

        - https://github.com/dhth/mrj/pull/1        head didn't match
        - https://github.com/dhth/mrj/pull/1        author unknown
        - https://github.com/dhth/mrj/pull/1        author untrusted-dependabot[bot] untrusted
        - https://github.com/dhth/mrj/pull/1        check lint: unknown conclusion
        - https://github.com/dhth/mrj/pull/1        check lint: failure
        - https://github.com/dhth/mrj/pull/1        state: unknown
        - https://github.com/dhth/mrj/pull/1        state: dirty
        "
    );
}

#[test]
fn summary_doesnt_include_dq_if_none_exist() {
    // GIVEN
    let mut buffer = vec![];

    let behaviours = RunBehaviours::default().skip_disqualifications_in_summary();

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
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");

    let (_, summary) = out
        .split_once(
            r#"
===========
  SUMMARY
===========
"#,
        )
        .expect("output should've been split by the summary header");

    assert_snapshot!(
        summary,
        @r"
        - PRs merged:                    0
        - PRs disqualified:              0
        - Repos checked:                 1
        - Repos with no relevant PRs:    0
        - Errors encountered:            0
        "
    );
}

#[test]
fn disqualification_reasons_are_left_aligned_in_summary() {
    // GIVEN
    let mut buffer = vec![];

    let behaviours = RunBehaviours::default().show_prs_with_unmatched_head();

    let mut l = RunLog::new(&mut buffer, &behaviours);
    let repo_check = RepoCheck {
        owner: OWNER.to_string(),
        name: REPO.to_string(),
        state: RepoCheckFinished(vec![
            merge_result_disqualified_unmatched_head(1),
            merge_result_disqualified_unmatched_head(11),
            merge_result_disqualified_unmatched_head(111),
            merge_result_disqualified_unmatched_head(1111),
        ]),
    };
    let repo_result = RepoResult::Finished(repo_check);

    // WHEN
    l.add_repo_result(repo_result);
    l.write_output().expect("output should've been written");

    // THEN
    let out =
        String::from_utf8(buffer).expect("buffer contents should've been converted to a string");

    let (_, summary) = out
        .split_once(
            r#"
===========
  SUMMARY
===========
"#,
        )
        .expect("output should've been split by the summary header");

    assert_snapshot!(
        summary,
        @r"
        - PRs merged:                    0
        - PRs disqualified:              4
        - Repos checked:                 1
        - Repos with no relevant PRs:    0
        - Errors encountered:            0

        Disqualifications
        ---

        - https://github.com/dhth/mrj/pull/1           head didn't match
        - https://github.com/dhth/mrj/pull/11          head didn't match
        - https://github.com/dhth/mrj/pull/111         head didn't match
        - https://github.com/dhth/mrj/pull/1111        head didn't match
        "
    );
}

fn merge_result_disqualified_unmatched_head(number: u64) -> MergeResult {
    MergeResult::Disqualified(PRCheck {
        number,
        title: PR_TITLE.to_string(),
        url: format!("https://github.com/dhth/mrj/pull/{number}"),
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
