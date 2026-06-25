//! Error types for the match-report-analyzer.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur while converting a match-report CSV into an Excel
/// workbook.
#[derive(Debug, Error)]
pub enum AppError {
    /// The input file is not a `.csv` file.
    #[error("input file {path} is not a .csv file (expected a .csv extension)")]
    NotCsv {
        /// The rejected input path.
        path: PathBuf,
    },

    /// The input file could not be read.
    #[error("failed to read input file {path}: {source}")]
    ReadInput {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The CSV is missing one or more columns required to build a match report.
    #[error("the CSV is missing required column(s): {}", .columns.join(", "))]
    MissingRequiredColumns {
        /// The names of the required columns that are absent.
        columns: Vec<String>,
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
