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
        "report", "generate", "-p", "log.txt", "-o", "-n", "20", "--debug",
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
    let mut cmd = fx.cmd(["report", "generate", "-n", "-10", "--debug"]);

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
    let mut cmd = fx.cmd(["report", "generate", "-n", "0", "--debug"]);

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
    let mut cmd = fx.cmd(["report", "generate", "-n", "101", "--debug"]);

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
