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
| Difference | 🟥 light red (`#FFC7CE`) | both values present but not equal |
| Missing | 🟨 light amber (`#FFEB9C`) | a value is present on one side only |
| _(none)_ | — | the values are equal, or the column is not a `REF_`/`CAN_` pair |

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
- **Grouped pair headers** — a two-row header groups each `REF_`/`CAN_` pair
  under a single band showing the field name once (e.g. `XUNITS` over a `REF`
  and a `CAN` sub-column), making the pairs easy to spot among the plain
  columns. Unpaired columns span both header rows.
- **Styled header** — bold white text on a deep blue band, frozen along with the
  first column so context stays visible while scrolling.
- **Sized columns** — each column is fit to its content (capped so long URLs
  don't dominate), with an autofilter over the data.

## Installation

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
