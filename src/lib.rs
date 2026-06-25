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

use std::path::Path;

use tracing::info;

pub use crate::error::AppError;
use crate::report::Report;
use crate::xlsx::ConversionStats;

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
