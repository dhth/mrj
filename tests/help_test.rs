use assert_cmd::Command;
use predicates::str::contains;

//-------------//
//  SUCCESSES  //
//-------------//

#[test]
fn shows_help() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("--help");

    // WHEN
    // THEN
    cmd.assert()
        .success()
        .stdout(contains("mrj merges your open PRs"));
}
