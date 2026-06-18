# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project scaffolding: licensing (MIT OR Apache-2.0), README, contributor
  guide, code of conduct, CI workflow, and editor/formatting configuration.
- Native desktop GUI built with the [Iced](https://iced.rs/) toolkit, targeting
  macOS and Windows.
- Open a match-report CSV via a native file picker; data is loaded read-only
  into an in-memory SQLite database with automatic column-type inference.
- Read-only result grid with click-to-sort column headers, a visually distinct
  (bold, shaded, bordered) header row, zebra-striped data rows, and
  drag-to-resize column widths.
- Two ways to query the data, both compiling to SQL against one table: a
  structured filter builder (column/operator/value conditions joined with
  AND/OR) and a raw SQL box.
- Split into a testable library (`store`, `query`) plus a thin GUI binary, with
  unit tests and a sample-data integration test.

[Unreleased]: https://github.com/jchultarsky101/match-report-analyzer/commits/main
