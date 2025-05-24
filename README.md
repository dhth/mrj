<p align="center">
  <h1 align="center">mrj</h1>
  <p align="center">
    <a href="https://github.com/dhth/mrj/actions/workflows/build.yml"><img alt="GitHub release" src="https://img.shields.io/github/actions/workflow/status/dhth/mrj/build.yml?style=flat-square"></a>
  </p>
</p>

`mrj` merges your open dependency upgrade PRs.

![usage](https://tools.dhruvs.space/images/mrj/mrj-1.png)

> [!NOTE]
> mrj is alpha software. Its interface and behaviour might change in the near
> future.

⚡️ Usage
---

```text
$ mrj -h
mrj merges your open PRs

Usage: mrj [OPTIONS] <COMMAND>

Commands:
  run     Check for open PRs and merge them
  config  Interact with mrj's config
  help    Print this message or the help of the given subcommand(s)

Options:
      --debug  Output debug information without doing anything
  -h, --help   Print help
```

```text
$ mrj run -h
Check for open PRs and merge them

Usage: mrj run [OPTIONS]

Options:
  -c, --config <PATH>          Path to mrj's config file [default: mrj.toml]
  -r, --repos <STRING,STRING>  Repos to run for (will override repos in config)
  -d, --dry-run                Whether to only print out information without merging any PRs
      --debug                  Output debug information without doing anything
  -h, --help                   Print help
```

📃 Config
---

`mrj` needs a config file to run. Here's a sample config.

```toml
# repos to run for
# (required)
repos = [
    "dhth/act3",
    "dhth/ecscope",
    "dhth/ecsv",
    "dhth/mrj",
    "dhth/shfl",
    "dhth/squidge",
    "dhth/squish",
    "dhth/tash",
    "dhth/tbll",
    "dhth/tomo",
    "dhth/urll",
]

# mrj will only consider repos created by the users in this list
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

# by default mrj only considers PRs where checks have either passed or are
# skipped
# if this setting is ON, mrj will not consider PRs where one or more check has
# been skipped
# (optional, default: true)
merge_if_checks_skipped = true

# squash/merge/rebase (make sure the choice is actually enabled in your
# settings)
# (required)
merge_type = "squash"
```
