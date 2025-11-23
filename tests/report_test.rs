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
    let mut cmd = fx.cmd(["report", "generate", "--debug"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    DEBUG INFO

    command:        Generate report
    output file:    output.txt
    open report:    false
    num runs:       10
    title:          mrj runs
    template path:  <NOT PROVIDED>

    ----- stderr -----
    ");
}

#[test]
fn overriding_flags_works() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd([
        "report",
        "generate",
        "--debug",
        "--html-template",
        "path/to/template.html",
        "--num-runs",
        "20",
        "--open",
        "--output-path",
        "log.txt",
        "--title",
        "dependency updates",
    ]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    DEBUG INFO

    command:        Generate report
    output file:    log.txt
    open report:    true
    num runs:       20
    title:          dependency updates
    template path:  path/to/template.html

    ----- stderr -----
    ");
}

//-------------//
//  FAILURES   //
//-------------//

#[test]
fn fails_if_num_runs_is_negative() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["report", "generate", "--debug", "--num-runs", "-10"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: unexpected argument '-1' found

    Usage: mrj report generate [OPTIONS]

    For more information, try '--help'.
    ");
}

#[test]
fn fails_if_num_runs_is_zero() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["report", "generate", "--debug", "--num-runs", "0"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value '0' for '--num-runs <NUMBER>': 0 is not in 1..=100

    For more information, try '--help'.
    ");
}

#[test]
fn fails_if_num_runs_is_greater_than_max_allowed() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["report", "generate", "--debug", "--num-runs", "101"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value '101' for '--num-runs <NUMBER>': 101 is not in 1..=100

    For more information, try '--help'.
    ");
}
