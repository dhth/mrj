mod common;

use common::Fixture;
use insta_cmd::assert_cmd_snapshot;

//-------------//
//  SUCCESSES  //
//-------------//

#[test]
fn debug_mode_works() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd([
        "run",
        "--debug",
        "-c",
        "tests/assets/valid-config-with-all-props.toml",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    DEBUG INFO

    command:                              Run
    config file:                          tests/assets/valid-config-with-all-props.toml
    repos (overridden):                   []
    output to file:                       false
    output file:                          output.txt
    write summary:                        false
    summary file:                         summary.txt
    skip disqualifications in summary:    false
    show repos with no prs:               false
    show prs from untrusted authors:      false
    show prs with unmatched head:         false
    execute:                              false
    plain stdout:                         false

    ----- stderr -----
    ");
}

#[test]
fn overriding_repos_works() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd([
        "run",
        "--debug",
        "-c",
        "tests/assets/valid-config-with-no-repos.toml",
        "--repos",
        "dhth/mrj,dhth/bmm",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    DEBUG INFO

    command:                              Run
    config file:                          tests/assets/valid-config-with-no-repos.toml
    repos (overridden):                   ["dhth/mrj", "dhth/bmm"]
    output to file:                       false
    output file:                          output.txt
    write summary:                        false
    summary file:                         summary.txt
    skip disqualifications in summary:    false
    show repos with no prs:               false
    show prs from untrusted authors:      false
    show prs with unmatched head:         false
    execute:                              false
    plain stdout:                         false

    ----- stderr -----
    "#);
}

//-------------//
//  FAILURES   //
//-------------//

#[test]
fn fails_if_overridden_repos_are_invalid() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd([
        "run",
        "--debug",
        "-c",
        "tests/assets/valid-config-with-all-props.toml",
        "--repos",
        "dhth/mrj,invalid-repo,dhth/bmm",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r#"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value 'invalid-repo' for '--repos <STRING,STRING>': repo needs to be in the form "owner/repo"

    For more information, try '--help'.
    "#);
}

#[test]
fn fails_if_no_repos_provided() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["run", "-c", "tests/assets/valid-config-with-no-repos.toml"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: no repos to run for
    ");
}
