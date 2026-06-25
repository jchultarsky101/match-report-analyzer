# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project scaffolding: licensing (MIT OR Apache-2.0), README, contributor
  guide, code of conduct, CI workflow, and editor/formatting configuration.
- Command-line tool that converts a Physna match-report CSV into a
  color-highlighted Excel (`.xlsx`) workbook, taking the input CSV and output
  `.xlsx` path as required arguments.
- Pairing of `REF_<field>` / `CAN_<field>` columns by their shared field name,
  with per-cell comparison highlighting both halves of a pair when they differ
  (light red) or when a value is present on only one side (light amber).
- Faithful text output (values written verbatim, no numeric coercion), a bold
  frozen header row, and an autofilter over the data range.
- Structured logging via [`tracing`](https://crates.io/crates/tracing), CLI
  parsing with [`clap`](https://crates.io/crates/clap) (builder pattern), and
  typed errors with [`thiserror`](https://crates.io/crates/thiserror).
- Help screen shown via a `help` subcommand and when no arguments are supplied,
  in addition to `-h`/`--help`.
- Automatic correction of the output file extension to `.xlsx` (with a warning)
  when a different or missing extension is given, so Excel opens the result
  without an "extension doesn't match" warning.
- Visual presentation of the workbook: rows sorted by `MATCH_PERCENTAGE`
  (highest first); a heat-map color scale on that column (cool blue at 0% →
  yellow at 50% → red at 100%, anchored to a fixed 0–100 range); a bold
  white-on-blue header frozen with the first column; per-content column widths
  (capped); a named worksheet; and document properties.
- Grouped two-row header: each `REF_`/`CAN_` pair is shown under a merged band
  labeled with the field name once, with `REF`/`CAN` sub-labels beneath, so the
  column pairs are easy to identify. Unpaired columns span both header rows.
- Split into a testable library (`cli`, `report`, `xlsx`, `error`) plus a thin
  binary, with unit tests for column pairing, cell classification, and argument
  parsing.

### Changed
- Replaced the earlier Iced-based desktop GUI prototype (SQLite-backed query
  grid) with this focused CSV-to-Excel CLI. None of the GUI work was released.

[Unreleased]: https://github.com/jchultarsky101/match-report-analyzer/commits/main
