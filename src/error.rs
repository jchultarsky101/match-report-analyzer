//! Error types for the match-report-analyzer.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur while converting a match-report CSV into an Excel
/// workbook.
#[derive(Debug, Error)]
pub enum AppError {
    /// The input file could not be read.
    #[error("failed to read input file {path}: {source}")]
    ReadInput {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// An error occurred while parsing the CSV.
    #[error("failed to parse CSV: {0}")]
    Csv(#[from] csv::Error),

    /// The CSV contained no header row, so columns cannot be interpreted.
    #[error("the CSV has no header row; cannot determine columns")]
    MissingHeaders,

    /// An error occurred while building or saving the Excel workbook.
    #[error("failed to write Excel workbook: {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
}
