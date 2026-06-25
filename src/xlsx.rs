//! Renders a [`Report`] into a color-highlighted Excel workbook.

use std::path::Path;

use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook};
use tracing::{debug, info};

use crate::error::AppError;
use crate::report::{CellState, Report};

/// Background color for cells whose reference and candidate values differ.
/// This is Excel's standard "bad" red fill.
const COLOR_DIFFERENT: Color = Color::RGB(0xFFC7CE);
/// Background color for cells where the value is present on one side but missing
/// on the other. This is Excel's standard "neutral" amber fill.
const COLOR_MISSING: Color = Color::RGB(0xFFEB9C);
/// Background color for the header row.
const COLOR_HEADER: Color = Color::RGB(0xD9D9D9);

/// A summary of what was written, returned for logging and reporting.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConversionStats {
    /// Number of data rows written.
    pub rows: usize,
    /// Number of comparable `REF_`/`CAN_` pairs in the schema.
    pub pairs: usize,
    /// Number of cells highlighted as differing.
    pub different: usize,
    /// Number of cells highlighted as missing-on-one-side.
    pub missing: usize,
}

/// Writes `report` to an `.xlsx` workbook at `output`, highlighting cells where
/// the reference and candidate metadata differ.
pub fn write_workbook(report: &Report, output: &Path) -> Result<ConversionStats, AppError> {
    let header_format = Format::new()
        .set_bold()
        .set_background_color(COLOR_HEADER)
        .set_border(FormatBorder::Thin);
    let different_format = Format::new().set_background_color(COLOR_DIFFERENT);
    let missing_format = Format::new().set_background_color(COLOR_MISSING);

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let schema = &report.schema;
    let mut stats = ConversionStats {
        pairs: schema.pair_count(),
        ..Default::default()
    };

    // Header row.
    for (col, label) in schema.headers().iter().enumerate() {
        worksheet.write_with_format(0, col as u16, label, &header_format)?;
    }

    // Data rows. The header occupies row 0, so data starts at row 1.
    for (row_idx, record) in report.rows.iter().enumerate() {
        let excel_row = (row_idx + 1) as u32;
        for col in 0..schema.column_count() {
            let value = record.get(col).map(String::as_str).unwrap_or("");
            match report.cell_state(row_idx, col) {
                CellState::Equal => {
                    worksheet.write(excel_row, col as u16, value)?;
                }
                CellState::Different => {
                    worksheet.write_with_format(excel_row, col as u16, value, &different_format)?;
                    stats.different += 1;
                }
                CellState::Missing => {
                    worksheet.write_with_format(excel_row, col as u16, value, &missing_format)?;
                    stats.missing += 1;
                }
            }
        }
    }
    stats.rows = report.rows.len();

    // Freeze the header row and enable an autofilter for easier inspection.
    worksheet.set_freeze_panes(1, 0)?;
    if schema.column_count() > 0 && !report.rows.is_empty() {
        let last_col = (schema.column_count() - 1) as u16;
        let last_row = report.rows.len() as u32;
        worksheet.autofilter(0, 0, last_row, last_col)?;
    }

    debug!(?output, "saving workbook");
    workbook.save(output)?;
    info!(
        rows = stats.rows,
        pairs = stats.pairs,
        different = stats.different,
        missing = stats.missing,
        "conversion complete"
    );

    Ok(stats)
}
