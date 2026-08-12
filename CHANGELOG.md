# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Allow merging PRs with neutral check conclusions via an optional configuration parameter

## [v0.5.0] - May 29, 2026

### Changed

- Replace raw run logs in the HTML report with structured run summaries and per-PR status tables
- (breaking) `mrj run --output-to-file` now writes structured JSON to `output.json` instead of plain text to `output.txt`

## [v0.4.0] - Nov 24, 2025

### Added

- List disqualifications in summary
- Allow printing to stdout without color

### Changed

- Default to skipping PRs where head doesn't match the configured pattern
- Allow overriding report template and title
- Include all runs in HTML report page

## [v0.3.0] - Jun 16, 2025

### Added

- Allow configuring sort by and sort direction for pull requests

### Changed

- Operate in dry-run mode by default

## [v0.2.0] - Jun 06, 2025

### Added

- Allow changing number of runs to keep in report

### Changed

- Ignore PRs from untrusted authors by default

[unreleased]: https://github.com/dhth/mrj/compare/v0.5.0...HEAD
[v0.5.0]: https://github.com/dhth/mrj/compare/v0.4.0...v0.5.0
[v0.4.0]: https://github.com/dhth/mrj/compare/v0.3.0...v0.4.0
[v0.3.0]: https://github.com/dhth/mrj/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/dhth/mrj/compare/v0.1.0...v0.2.0
