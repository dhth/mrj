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
        "run",
        "--debug",
        "-c",
        "tests/assets/valid-config-with-all-props.toml",
    ]);

    // WHEN
    // THEN
    cmd.assert().success().stdout(contains("DEBUG INFO"));
}

#[test]
fn overriding_repos_works() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args([
        "run",
        "--debug",
        "-c",
        "tests/assets/valid-config-with-no-repos.toml",
        "--repos",
        "dhth/mrj,dhth/bmm",
    ]);

    // WHEN
    // THEN
    cmd.assert().success().stdout(contains("DEBUG INFO"));
}

//-------------//
//  FAILURES   //
//-------------//

#[test]
fn fails_if_overridden_repos_are_invalid() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args([
        "run",
        "--debug",
        "-c",
        "tests/assets/valid-config-with-all-props.toml",
        "--repos",
        "dhth/mrj,invalid-repo,dhth/bmm",
    ]);

    // WHEN
    // THEN
    cmd.assert().failure();
}

#[test]
fn fails_if_no_repos_provided() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args([
        "run",
        "--dry-run",
        "-c",
        "tests/assets/valid-config-with-no-repos.toml",
    ]);

    // WHEN
    // THEN
    cmd.assert()
        .failure()
        .stderr(contains("no repos to run for"));
}
