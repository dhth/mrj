use assert_cmd::Command;
use predicates::str::contains;

//-------------//
//  SUCCESSES  //
//-------------//

#[test]
fn parsing_valid_config_with_all_props_works() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args([
        "config",
        "validate",
        "-p",
        "tests/assets/valid-config-with-all-props.toml",
    ]);

    // WHEN
    // THEN
    cmd.assert().success().stdout(contains("config looks good"));
}

#[test]
fn parsing_valid_config_with_mandatory_props_only_works() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args([
        "config",
        "validate",
        "-p",
        "tests/assets/valid-config-with-mandatory-props-only.toml",
    ]);

    // WHEN
    // THEN
    cmd.assert().success().stdout(contains("config looks good"));
}

#[test]
fn printing_sample_config_works() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args(["config", "sample"]);

    // WHEN
    // THEN
    cmd.assert().success().stdout(contains("# mrj.toml"));
}

//-------------//
//  FAILURES   //
//-------------//

#[test]
fn parsing_invalid_toml_fails() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args(["config", "validate", "-p", "tests/assets/invalid.toml"]);

    // WHEN
    // THEN
    cmd.assert().failure();
}

#[test]
fn parsing_invalid_config_fails() {
    // GIVEN
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args([
        "config",
        "validate",
        "-p",
        "tests/assets/invalid-config.toml",
    ]);

    // WHEN
    // THEN
    cmd.assert().failure();
}
