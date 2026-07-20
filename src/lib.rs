//! # match-report-analyzer
//!
//! Analyzes a Physna match-report CSV and renders it into other formats.
//!
//! A match report compares the metadata of a *reference* asset against a
//! *candidate* asset. Paired metadata columns are named `REF_<field>` and
//! `CAN_<field>`. Two renderings are available:
//!
//! - [`convert_to_xlsx`] — a color-highlighted Excel workbook flagging, cell
//!   by cell, where the paired values differ ([red][crate::xlsx]) or are
//!   present on only one side (amber).
//! - [`convert_to_graph`] — an interactive, self-contained HTML
//!   "constellation" [graph] in which each asset is a node and each match an
//!   edge, revealing clusters of similar assets across the whole report.
//! - [`convert_to_grid`] — an interactive, self-contained HTML data [grid]
//!   mirroring the Excel view, with search, sorting, a SQL-like filter, and
//!   in-place "what-if" editing.

pub mod cli;
pub mod error;
pub mod graph;
pub mod grid;
pub mod html;
mod json;
pub mod report;
pub mod xlsx;

use std::path::{Path, PathBuf};

use tracing::info;

pub use crate::error::AppError;
use crate::graph::Graph;
use crate::grid::GridStats;
use crate::html::GraphStats;
use crate::report::Report;
use crate::xlsx::ConversionStats;

/// Ensures the output path carries the given extension.
///
/// Each subcommand writes exactly one format, and the reading application
/// (Excel, a browser) keys off the extension. If the caller supplies a
/// different or missing extension — most commonly the legacy `.xls` for the
/// Excel output — we coerce it rather than make the user guess.
///
/// The comparison is case-insensitive, so an existing `.XLSX` is left
/// untouched. Returns the path that should actually be written.
pub fn normalize_output_path(path: &Path, extension: &str) -> PathBuf {
    let already_correct = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension));

    if already_correct {
        path.to_path_buf()
    } else {
        path.with_extension(extension)
    }
}

/// File extension required of the input file.
const CSV_EXTENSION: &str = "csv";

/// Reads and validates the match-report CSV at `input`.
///
/// The input is rejected when it isn't a `.csv` file, can't be parsed, or is
/// missing one of the required columns. A CSV with no `REF_`/`CAN_` metadata
/// pairs is perfectly valid.
fn load_report(input: &Path) -> Result<Report, AppError> {
    // Reject anything that isn't a .csv file outright.
    let is_csv = input
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(CSV_EXTENSION));
    if !is_csv {
        return Err(AppError::NotCsv {
            path: input.to_path_buf(),
        });
    }

    info!(?input, "reading match report");
    let report = Report::from_csv_path(input)?;
    info!(
        rows = report.rows.len(),
        columns = report.schema.column_count(),
        pairs = report.schema.pair_count(),
        "parsed report"
    );

    // Reject files that lack the columns a match report must have.
    let missing = report.schema.missing_required_columns();
    if !missing.is_empty() {
        return Err(AppError::MissingRequiredColumns {
            columns: missing.into_iter().map(String::from).collect(),
        });
    }

    Ok(report)
}

/// Reads the match-report CSV at `input` and writes a highlighted `.xlsx`
/// workbook to `output`, returning a summary of what was written.
pub fn convert_to_xlsx(input: &Path, output: &Path) -> Result<ConversionStats, AppError> {
    let mut report = load_report(input)?;

    // Surface the most relevant pairs first: sort by match percentage, highest
    // (closest to an identical match) at the top. MATCH_PERCENTAGE is required,
    // so it is always present here.
    if let Some(column) = report.schema.column_index(report::MATCH_PERCENTAGE_COLUMN) {
        report.sort_by_numeric_desc(column);
        info!(
            column = report::MATCH_PERCENTAGE_COLUMN,
            "sorted rows descending"
        );
    }

    xlsx::write_workbook(&report, output)
}

/// Reads the match-report CSV at `input` and writes an interactive HTML
/// similarity-graph document to `output`, returning a summary of the graph.
pub fn convert_to_graph(input: &Path, output: &Path) -> Result<GraphStats, AppError> {
    let report = load_report(input)?;

    let graph = Graph::from_report(&report);
    info!(
        nodes = graph.nodes.len(),
        edges = graph.edges.len(),
        "built similarity graph"
    );

    html::write_document(&graph, &source_name(input), output)
}

/// Reads the match-report CSV at `input` and writes an interactive HTML
/// data-grid document to `output`, returning a summary of what was written.
pub fn convert_to_grid(input: &Path, output: &Path) -> Result<GridStats, AppError> {
    let mut report = load_report(input)?;

    // Initial order mirrors the Excel view: most relevant pairs first. The
    // grid re-sorts interactively from there.
    if let Some(column) = report.schema.column_index(report::MATCH_PERCENTAGE_COLUMN) {
        report.sort_by_numeric_desc(column);
        info!(
            column = report::MATCH_PERCENTAGE_COLUMN,
            "sorted rows descending"
        );
    }

    grid::write_document(&report, &source_name(input), output)
}

/// A human-readable name for the input file, shown in generated page headers.
fn source_name(input: &Path) -> String {
    input
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| input.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_existing_matching_extension() {
        assert_eq!(
            normalize_output_path(Path::new("report.xlsx"), "xlsx"),
            PathBuf::from("report.xlsx")
        );
        assert_eq!(
            normalize_output_path(Path::new("graph.html"), "html"),
            PathBuf::from("graph.html")
        );
    }

    #[test]
    fn replaces_wrong_extension() {
        assert_eq!(
            normalize_output_path(Path::new("data/test-report.xls"), "xlsx"),
            PathBuf::from("data/test-report.xlsx")
        );
        assert_eq!(
            normalize_output_path(Path::new("graph.htm"), "html"),
            PathBuf::from("graph.html")
        );
    }

    #[test]
    fn adds_extension_when_missing() {
        assert_eq!(
            normalize_output_path(Path::new("report"), "xlsx"),
            PathBuf::from("report.xlsx")
        );
    }

    #[test]
    fn existing_extension_match_is_case_insensitive() {
        assert_eq!(
            normalize_output_path(Path::new("report.XLSX"), "xlsx"),
            PathBuf::from("report.XLSX")
        );
    }
}
