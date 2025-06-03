use assert_cmd::Command;
use predicates::str::contains;

//-------------//
//  SUCCESSES  //
//-------------//

#[test]
fn debug_mode_works() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args([
        "report", "generate", "-p", "log.txt", "-o", "-n", "20", "--debug",
    ]);

    // WHEN
    // THEN
    cmd.assert().success().stdout(contains("DEBUG INFO"));
}

//-------------//
//  FAILURES   //
//-------------//

#[test]
fn fails_if_num_runs_is_invalid() {
    // GIVEN
    let test_cases = [-10, 0, 101];
    for num_run in test_cases {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.args(["report", "generate", "-n", &num_run.to_string(), "--debug"]);

        // WHEN
        // THEN
        cmd.assert().failure();
    }
}
