# Contributing to match-report-analyzer

Thanks for your interest in contributing! This project is in early development,
and contributions of all kinds — bug reports, feature ideas, documentation, and
code — are welcome.

By participating, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting started

1. Install a recent Rust toolchain (edition 2024, Rust **1.85+**) via
   [rustup](https://rustup.rs/).
2. Fork and clone the repository.
3. Build and test:

   ```sh
   cargo build
   cargo test
   ```

## Before opening a pull request

Please make sure the following all pass locally — CI enforces them:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

## Guidelines

- **Keep changes focused.** One logical change per pull request.
- **Write tests** for new behavior and bug fixes where practical.
- **Document public APIs** with `///` doc comments and update `README.md` when
  user-facing behavior changes.
- **Update [CHANGELOG.md](CHANGELOG.md)** under the `[Unreleased]` section.
- **Don't commit sample data.** The `data/` directory is git-ignored; never add
  customer or proprietary CSVs to the repository. See [`data/README.md`](data/README.md).

## Commit messages

Write clear, imperative commit messages (e.g. "Add similarity threshold filter").
[Conventional Commits](https://www.conventionalcommits.org/) are encouraged but
not required.

## Licensing of contributions

Unless you state otherwise, any contribution you submit is dual licensed under
the MIT and Apache-2.0 licenses, matching the project's [license](README.md#license).
