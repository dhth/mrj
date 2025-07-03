mod common;

use common::Fixture;
use insta_cmd::assert_cmd_snapshot;

//-------------//
//  SUCCESSES  //
//-------------//

#[test]
fn shows_help() {
    // GIVEN
    let fx = Fixture::new();
    let mut cmd = fx.cmd(["--help"]);

    // WHEN
    // THEN
    assert_cmd_snapshot!(cmd, @r"
    success: true
    exit_code: 0
    ----- stdout -----
    mrj merges your open PRs

    Usage: mrj [OPTIONS] <COMMAND>

    Commands:
      run     Check for open PRs and merge them
      config  Interact with mrj's config
      report  Generate report from mrj runs
      help    Print this message or the help of the given subcommand(s)

    Options:
          --debug  Output debug information without doing anything
      -h, --help   Print help

    ----- stderr -----
    ");
}
