# match-report-analyzer

[![CI](https://github.com/jchultarsky101/match-report-analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/jchultarsky101/match-report-analyzer/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A small **command-line tool** that converts a
[Physna](https://www.physna.com/) geometric **match-report** CSV export into a
**color-highlighted Excel workbook** (`.xlsx`).

Each row of a match report pairs a *reference* asset with a *candidate* asset.
The tool highlights, cell by cell, **where their metadata differs** so you can
scan a large report visually instead of reading every value by hand.

> **Status:** early development (`0.1.0`).

## How it works

Paired metadata lives in columns named with a prefix:

- `REF_<field>` — the reference asset's value for `<field>`
- `CAN_<field>` — the candidate asset's value for the **same** `<field>`

The text after the prefix is the actual field name and must match between the
two columns (for example `REF_XUNITS` ↔ `CAN_XUNITS`, or
`REF__COST_($)` ↔ `CAN__COST_($)`). All other columns
(`REFERENCE_ASSET_PATH`, `MATCH_PERCENTAGE`, `COMPARISON_URL`, …) are copied
through unchanged.

For every `REF_`/`CAN_` pair in each row the tool compares the two values and
highlights **both** cells:

| Highlight | Color | Meaning |
| --- | --- | --- |
| Match | 🟩 light green (`#C6EFCE`) | both values present and equal |
| Difference | 🟥 light red (`#FFC7CE`) | both values present but not equal |
| Missing | 🟨 light amber (`#FFEB9C`) | a value is present on one side only |
| _(none)_ | — | not a `REF_`/`CAN_` pair, or both values are empty |

Highlighting only applies to `REF_`/`CAN_` pair columns; stand-alone columns are
never colored.

Values are compared as **trimmed text** and written to Excel verbatim, so the
exact CSV content is preserved (no numeric coercion, no lost leading zeros or
precision). The one exception is the `MATCH_PERCENTAGE` column, which is written
as a real number so it can be sorted and shaded numerically.

### Presentation

The generated workbook is styled for readability:

- **Sorted by relevance** — rows are ordered by `MATCH_PERCENTAGE`, highest
  (closest to an identical match) first.
- **Heat-map gradient** on the `MATCH_PERCENTAGE` column — a color scale from a
  cool blue at 0%, through yellow at 50%, to "red hot" at 100%, so the strongest
  matches stand out at a glance.
- **Grouped & boxed pair headers** — a two-row header groups each `REF_`/`CAN_`
  pair under a single band showing the field name once (e.g. `XUNITS` over a
  `REF` and a `CAN` sub-column), and a medium border is drawn around each pair
  from the header down to the last row — so the pairs are unmistakable among the
  plain columns. Unpaired columns span both header rows.
- **Styled header** — bold white text on a deep blue band, frozen along with the
  three asset-pair identity columns (reference path, candidate path, and match
  percentage) so the pair each row describes stays visible while scrolling.
- **Sized columns** — each column is fit to its content (capped so long URLs
  don't dominate), with an autofilter over the data.
- **Clickable comparison link** — the `COMPARISON_URL` column is written as a
  hyperlink (and sized to fit its values, up to Excel's maximum column width), so
  you can click straight through to the side-by-side comparison in a browser.

## Installation

### Pre-built binaries (recommended)

Once a release has been published, install the latest version with one command.
Pre-built binaries are provided for Windows (x64), macOS (Intel & Apple Silicon),
and Linux (x64).

**macOS / Linux:**

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jchultarsky101/match-report-analyzer/releases/latest/download/match-report-analyzer-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/jchultarsky101/match-report-analyzer/releases/latest/download/match-report-analyzer-installer.ps1 | iex"
```

The installers also place a `match-report-analyzer-update` helper on your `PATH`;
run it at any time to upgrade an existing install to the newest release.

### From source

Requires a recent Rust toolchain (edition 2024, Rust **1.85+**). Install via
[rustup](https://rustup.rs/).

```sh
git clone https://github.com/jchultarsky101/match-report-analyzer.git
cd match-report-analyzer
cargo build --release
# Binary at ./target/release/match-report-analyzer (.exe on Windows)
```

## Usage

```sh
match-report-analyzer <INPUT_CSV> <OUTPUT_XLSX>
```

For example:

```sh
match-report-analyzer data/test-report.csv report.xlsx
```

Both arguments are required: the input match-report CSV and the path of the
`.xlsx` file to create. The input file is only read, never modified.

The input is validated before any work is done:

- It must be a `.csv` file, otherwise it is rejected.
- It must contain the `REFERENCE_ASSET_PATH`, `CANDIDATE_ASSET_PATH`, and
  `MATCH_PERCENTAGE` columns; if any is missing the file is rejected.

A CSV with no `REF_`/`CAN_` metadata pairs is perfectly valid — it is still
converted to a workbook normally, just with nothing to highlight.

> The output is always written in the modern Excel `.xlsx` format. If you give
> the output a different (or missing) extension — for example the legacy
> `.xls` — the tool automatically corrects it to `.xlsx` (and logs a warning),
> so Excel can open the file without an "extension doesn't match" warning.

Options:

| Flag | Description |
| --- | --- |
| `-v`, `--verbose` | Increase logging verbosity (`-v` = debug, `-vv` = trace) |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

Logging is powered by [`tracing`](https://crates.io/crates/tracing); set the
`RUST_LOG` environment variable for fine-grained control (it overrides `-v`).

## Architecture

The crate is a thin CLI binary over a small, testable library:

- `src/cli.rs` — argument parsing with [`clap`](https://crates.io/crates/clap)
  (builder pattern).
- `src/report.rs` — reads the CSV, pairs `REF_`/`CAN_` columns, and classifies
  each cell as equal / different / missing.
- `src/xlsx.rs` — writes the highlighted workbook with
  [`rust_xlsxwriter`](https://crates.io/crates/rust_xlsxwriter).
- `src/error.rs` — error types built with
  [`thiserror`](https://crates.io/crates/thiserror).
- `src/lib.rs` / `src/main.rs` — the `convert` entry point and the binary.

## Development

```sh
cargo build            # build
cargo test             # run tests
cargo fmt --all        # format
cargo clippy --all-targets --all-features -- -D warnings   # lint
```

Sample/test data lives in `data/` and is **git-ignored** — it is not distributed
with the repository. See [`data/README.md`](data/README.md).

### Releasing

Releases are built automatically by [dist](https://github.com/axodotdev/cargo-dist)
(see [`dist-workspace.toml`](dist-workspace.toml) and
[`.github/workflows/release.yml`](.github/workflows/release.yml)). To cut a
release:

1. Bump `version` in `Cargo.toml` and update `CHANGELOG.md`.
2. Commit, then tag and push:
   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

Pushing a `v*` tag triggers the release workflow, which cross-compiles the
binaries for all targets and publishes a GitHub Release with the archives,
checksums, install scripts, and updater. If you change anything in
`dist-workspace.toml`, run `dist generate` to refresh the workflow.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) and our
[Code of Conduct](CODE_OF_CONDUCT.md) before opening an issue or pull request.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
