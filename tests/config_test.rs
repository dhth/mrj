mod common;

use common::base_command;
use insta_cmd::assert_cmd_snapshot;

//-------------//
//  SUCCESSES  //
//-------------//

#[test]
fn parsing_valid_config_with_all_props_works() {
    // GIVEN
    let mut base_cmd = base_command();
    let mut cmd = base_cmd.args([
        "config",
        "validate",
        "--path",
        "tests/assets/valid-config-with-all-props.toml",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    config looks good ✅

    ----- stderr -----
    ");
}

#[test]
fn parsing_valid_config_with_mandatory_props_only_works() {
    // GIVEN
    let mut base_cmd = base_command();
    let mut cmd = base_cmd.args([
        "config",
        "validate",
        "--path",
        "tests/assets/valid-config-with-mandatory-props-only.toml",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    config looks good ✅

    ----- stderr -----
    ");
}

#[test]
fn sample_config_is_valid() {
    // GIVEN
    let mut base_cmd = base_command();
    let mut cmd = base_cmd.args([
        "config",
        "validate",
        "--path",
        "src/assets/sample-config.toml",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    config looks good ✅

    ----- stderr -----
    ");
}

#[test]
fn printing_sample_config_works() {
    // GIVEN
    let mut base_cmd = base_command();
    let mut cmd = base_cmd.args(["config", "sample"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    # mrj.toml

    # repos to run for
    # (required)
    repos = [
        "owner/repo-1",
        "owner/repo-2",
        "owner/repo-3",
    ]

    # mrj will only consider repos created by the authors in this list
    # (required)
    trusted_authors = ["dependabot[bot]"]

    # by default mrj doesn't filter PRs by base branches
    # (optional, default: empty)
    base_branch = "main"

    # by default mrj doesn't filter PRs by head refs
    # read more on this here
    # https://docs.github.com/en/rest/pulls/pulls?apiVersion=2022-11-28#list-pull-requests--parameters
    # the value needs to be valid regex
    # (optional, default: empty)
    head_pattern = "(dependabot|update)"

    # by default mrj only considers PRs which can be cleanly merged
    # if this setting is ON, mrj will also consider PRs where merging is blocked
    # (optional, default: false)
    merge_if_blocked = true

    # by default mrj only considers PRs where checks have either passed or are skipped
    # if this setting is OFF, mrj will not merge PRs where one or more checks have been skipped
    # (optional, default: true)
    merge_if_checks_skipped = true

    # how to merge the pull request
    # can be one of: [squash, merge, rebase]
    # make sure the choice is actually enabled in your settings
    # (required)
    merge_type = "squash"

    # what to sort pull requests by
    # can be one of: created, updated, popularity, long-running
    # "popularity" will sort by the number of comments
    # "long-running" will sort by date created and will limit the results to pull
    # requests that have been open for more than a month and have had activity within
    # the past month
    #
    # (optional; default: created)
    sort_by = "created"

    # the direction of the sort
    # can be one of: [asc, desc]
    # (optional; default: asc)
    sort_direction = "asc"

    ----- stderr -----
    "###);
}

//-------------//
//  FAILURES   //
//-------------//

#[test]
fn parsing_invalid_toml_fails() {
    // GIVEN
    let mut base_cmd = base_command();
    let mut cmd = base_cmd.args(["config", "validate", "--path", "tests/assets/invalid.toml"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: TOML parse error at line 10, column 1
       |
    10 | trusted_authors = ["dependabot[bot]"]
       | ^
    invalid string
    expected `"`, `'`
    "###);
}

#[test]
fn parsing_invalid_config_fails() {
    // GIVEN
    let mut base_cmd = base_command();
    let mut cmd = base_cmd.args([
        "config",
        "validate",
        "--path",
        "tests/assets/invalid-config.toml",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: TOML parse error at line 1, column 9
      |
    1 | repos = "not a list"
      |         ^^^^^^^^^^^^
    invalid type: string "not a list", expected a sequence
    "###);
}

#[test]
fn fails_if_invalid_repos_provided_via_config() {
    // GIVEN
    let mut base_cmd = base_command();
    let mut cmd = base_cmd.args([
        "config",
        "validate",
        "--path",
        "tests/assets/config-with-invalid-repos.toml",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: TOML parse error at line 3, column 5
      |
    3 |     "invalid-repo",
      |     ^^^^^^^^^^^^^^
    invalid value: string "invalid-repo", expected a value in the form "owner/repo"
    "###);
}
