## Common Commands
- Prefer `just` over raw `cargo` commands.
- Build: `just build`
- Check: `just check`
- Test: `just test`
- Lint: `just lint`
- Format: `just fmt`
- Full local verification: `just all`
- Review snapshot changes: `just review`
- Run CLI locally: `just run -- --help`

## Codebase Layout
- CLI entrypoint: `src/main.rs`
- CLI parsing: `src/args.rs`
- Merge flow: `src/merge/`
- Report generation: `src/report/`
- Config parsing and sample config: `src/config.rs`, `src/assets/sample-config.toml`
- Integration tests: `tests/`

## Key Conventions
- Keep changes small and aligned with the existing module split instead of introducing new abstractions.
- Use `anyhow::Result` and `anyhow::Context` for fallible flows; keep user-facing errors descriptive.
- Clippy forbids `unwrap()` and `expect()` via `Cargo.toml`; propagate or handle errors instead.
- Preserve the existing CLI/help text style when changing commands or flags; integration tests snapshot command output.
- Prefer snapshot-based assertions for CLI and report output; update snapshots intentionally and review them before accepting.
- Keep sample data and HTML/template fixtures under existing `src/report/testdata/`, `src/report/assets/`, and `src/merge/assets/` locations.

## Testing
- Run `just test` for normal coverage.
- Run `just review` when snapshot tests change.
- Target a specific test file with `cargo test --test <name>`.
- Target a specific module test with `cargo test <filter>`.

## Release And Packaging
- Dist/release settings live in `dist-workspace.toml`, `rust-toolchain.toml`, and GitHub workflows under `.github/workflows/`.
