# match-report-analyzer

[![CI](https://github.com/jchultarsky101/match-report-analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/jchultarsky101/match-report-analyzer/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A cross-platform **native desktop application** for analyzing
[Physna](https://www.physna.com/) geometric **match-report** CSV exports — the
reports that pair a *reference* asset with one or more *candidate* assets and
score how geometrically similar they are.

It helps you turn a raw export into answers: which parts are likely duplicates,
which candidates clear a similarity threshold, and how matches are distributed
across folders, file types, and cost.

Built in Rust with the [Iced](https://iced.rs/) GUI toolkit, it runs as a single
native binary on **macOS** and **Windows** (and Linux).

> **Status:** early development (`0.1.0`). The UI and feature set are not yet
> stable.

## Why

A match report can contain thousands of reference/candidate pairs with columns
for match percentage, asset paths, units, file types, folders, owners, file
size, cost, supplier, and a deep-link comparison URL. Reading that by hand in a
spreadsheet is slow. This app loads the export and lets you filter, sort, and
summarize it through a point-and-click interface.

### Input format

The tool consumes the CSV produced by Physna's match report. Each row is a
reference→candidate pair; key columns include:

| Column | Meaning |
| --- | --- |
| `REFERENCE_ASSET_PATH` / `CANDIDATE_ASSET_PATH` | Paths of the two compared assets |
| `MATCH_PERCENTAGE` | Geometric similarity score (0–100) |
| `REF_X*` / `CAN_X*` | Per-asset metadata (units, file type, folder, owner, size, …) |
| `REF__COST_($)` / `CAN__COST_($)` | Cost associated with each asset |
| `COMPARISON_URL` | Deep link to the side-by-side comparison in Physna |

See [`data/README.md`](data/README.md) for how to supply your own sample files.

## Installation

Requires a recent Rust toolchain (edition 2024, Rust **1.85+**). Install via
[rustup](https://rustup.rs/).

```sh
# From source
git clone https://github.com/jchultarsky101/match-report-analyzer.git
cd match-report-analyzer
cargo build --release
# Binary at ./target/release/match-report-analyzer (.exe on Windows)
```

> Iced renders with wgpu, so a GPU with a working Vulkan/Metal/DX12 driver is
> recommended. On platforms without one it falls back to software rendering.

Packaged installers (`.dmg` / `.msi`) are planned for tagged releases.

## Usage

Launch the app:

```sh
cargo run --release        # during development
# or run the built binary directly
```

Then use **Open report…** to load a Physna match-report CSV. The UI for
filtering, sorting, and summarizing is under active development and will be
documented here as it lands.

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
