//! # match-report-analyzer
//!
//! Converts a Physna match-report CSV into a color-highlighted Excel workbook.
//!
//! A match report compares the metadata of a *reference* asset against a
//! *candidate* asset. Paired metadata columns are named `REF_<field>` and
//! `CAN_<field>`; the converter highlights every cell where the two values
//! differ ([red][crate::xlsx]) or where a value is present on only one side
//! (amber).

pub mod cli;
pub mod error;
pub mod report;
pub mod xlsx;

use std::path::{Path, PathBuf};

use tracing::info;

pub use crate::error::AppError;
use crate::report::Report;
use crate::xlsx::ConversionStats;

/// The only file extension Excel recognizes for the format this tool writes.
const XLSX_EXTENSION: &str = "xlsx";

/// Ensures the output path carries the `.xlsx` extension.
///
/// This tool always writes the modern Office Open XML format. If the caller
/// supplies a different or missing extension — most commonly the legacy `.xls` —
/// Excel refuses to open the file cleanly, warning that the extension and
/// contents disagree. Rather than make the user guess the correct extension, we
/// coerce it to `.xlsx`.
///
/// The comparison is case-insensitive, so an existing `.XLSX` is left untouched.
/// Returns the path that should actually be written.
pub fn normalize_output_path(path: &Path) -> PathBuf {
    let already_xlsx = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(XLSX_EXTENSION));

    if already_xlsx {
        path.to_path_buf()
    } else {
        path.with_extension(XLSX_EXTENSION)
    }
}

/// Reads the match-report CSV at `input` and writes a highlighted `.xlsx`
/// workbook to `output`.
pub fn convert(input: &Path, output: &Path) -> Result<ConversionStats, AppError> {
    info!(?input, "reading match report");
    let report = Report::from_csv_path(input)?;
    info!(
        rows = report.rows.len(),
        columns = report.schema.column_count(),
        pairs = report.schema.pair_count(),
        "parsed report"
    );
    xlsx::write_workbook(&report, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_existing_xlsx_extension() {
        assert_eq!(
            normalize_output_path(Path::new("report.xlsx")),
            PathBuf::from("report.xlsx")
        );
    }

    #[test]
    fn replaces_legacy_xls_extension() {
        assert_eq!(
            normalize_output_path(Path::new("data/test-report.xls")),
            PathBuf::from("data/test-report.xlsx")
        );
    }

    #[test]
    fn adds_extension_when_missing() {
        assert_eq!(
            normalize_output_path(Path::new("report")),
            PathBuf::from("report.xlsx")
        );
    }

    #[test]
    fn existing_extension_match_is_case_insensitive() {
        assert_eq!(
            normalize_output_path(Path::new("report.XLSX")),
            PathBuf::from("report.XLSX")
        );
    }
}
