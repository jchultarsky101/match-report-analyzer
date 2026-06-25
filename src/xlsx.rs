//! Renders a [`Report`] into a color-highlighted, visually polished Excel
//! workbook.
//!
//! Design choices follow common spreadsheet-presentation best practices: a
//! single accent color for the header, a restrained set of semantic highlight
//! colors, right-aligned numbers with a fixed precision, sized columns, a frozen
//! header, an autofilter, and a heat-map color scale over the key metric.

use std::path::Path;

use rust_xlsxwriter::{
    Color, ConditionalFormat3ColorScale, ConditionalFormatType, DocProperties, Format, FormatAlign,
    FormatBorder, Url, Workbook,
};
use tracing::{debug, info};

use crate::error::AppError;
use crate::report::{COMPARISON_URL_COLUMN, CellState, MATCH_PERCENTAGE_COLUMN, Report, Schema};

/// Excel's maximum supported URL length. Longer values are written as plain
/// text rather than risking a write error.
const MAX_URL_LEN: usize = 2080;

/// Background color for paired cells whose values match. Excel's standard
/// "good" green fill.
const COLOR_MATCH: Color = Color::RGB(0xC6EFCE);
/// Background color for cells whose reference and candidate values differ.
/// Excel's standard "bad" red fill.
const COLOR_DIFFERENT: Color = Color::RGB(0xFFC7CE);
/// Background color for cells where the value is present on one side but missing
/// on the other. Excel's standard "neutral" amber fill.
const COLOR_MISSING: Color = Color::RGB(0xFFEB9C);
/// Header fill: a deep, professional blue with white text for strong contrast.
/// Used for the top "group" header band and for unpaired column headers.
const COLOR_HEADER_BG: Color = Color::RGB(0x1F4E78);
/// Fill for the second header row (the `REF`/`CAN` sub-labels): a lighter blue
/// so the two header tiers are visually distinct.
const COLOR_SUBHEADER_BG: Color = Color::RGB(0x2E6CA4);

/// Heat-map color for the lowest match percentage (0%): a calm, "cool" blue.
const COLOR_HEAT_LOW: Color = Color::RGB(0x5A8AC6);
/// Heat-map color for the midpoint (50%): a warm yellow.
const COLOR_HEAT_MID: Color = Color::RGB(0xFFEB84);
/// Heat-map color for the highest match percentage (100%): "red hot".
const COLOR_HEAT_HIGH: Color = Color::RGB(0xF8696B);

/// Number format applied to the match-percentage column.
const PERCENT_NUM_FORMAT: &str = "0.00";

/// Height (in points) of each of the two header rows.
const HEADER_ROW_HEIGHT: f64 = 22.0;
/// Row index of the top "group" header (field-name band over each pair).
const GROUP_HEADER_ROW: u32 = 0;
/// Row index of the second header (per-column `REF`/`CAN` and unpaired names).
const LABEL_HEADER_ROW: u32 = 1;
/// Row index where data begins (after the two header rows).
const DATA_START_ROW: u32 = 2;
/// Padding (in characters) added to a column's widest content.
const COL_WIDTH_PADDING: f64 = 2.0;
/// Narrowest a sized column may be.
const MIN_COL_WIDTH: f64 = 8.0;
/// Widest a normal sized column may be, so long values don't dominate.
const MAX_COL_WIDTH: f64 = 48.0;
/// Excel's hard maximum column width, used for the comparison-URL column so it
/// is sized to its (long) values rather than the usual cap.
const EXCEL_MAX_COL_WIDTH: f64 = 255.0;

/// Worksheet tab name.
const SHEET_NAME: &str = "Match Report";

/// A summary of what was written, returned for logging and reporting.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConversionStats {
    /// Number of data rows written.
    pub rows: usize,
    /// Number of comparable `REF_`/`CAN_` pairs in the schema.
    pub pairs: usize,
    /// Number of cells highlighted as matching.
    pub matching: usize,
    /// Number of cells highlighted as differing.
    pub different: usize,
    /// Number of cells highlighted as missing-on-one-side.
    pub missing: usize,
}

/// Writes `report` to an `.xlsx` workbook at `output`, highlighting cells where
/// the reference and candidate metadata differ and applying a heat-map gradient
/// to the match-percentage column.
pub fn write_workbook(report: &Report, output: &Path) -> Result<ConversionStats, AppError> {
    // Pair band (top header row over each pair): medium top/left/right border to
    // begin the box drawn around the pair, thin bottom against the sub-labels.
    let band_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(COLOR_HEADER_BG)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_top(FormatBorder::Medium)
        .set_border_left(FormatBorder::Medium)
        .set_border_right(FormatBorder::Medium)
        .set_border_bottom(FormatBorder::Thin);
    // Unpaired column headers span both header rows with a plain thin border.
    let unpaired_header_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(COLOR_HEADER_BG)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let percent_format = Format::new().set_num_format(PERCENT_NUM_FORMAT);

    let mut workbook = Workbook::new();
    workbook.set_properties(
        &DocProperties::new()
            .set_title("Physna Match Report")
            .set_subject("Reference vs. candidate metadata comparison"),
    );

    let worksheet = workbook.add_worksheet();
    worksheet.set_name(SHEET_NAME)?;

    let schema = &report.schema;
    let match_col = schema.column_index(MATCH_PERCENTAGE_COLUMN);
    let url_col = schema.column_index(COMPARISON_URL_COLUMN);
    let mut stats = ConversionStats {
        pairs: schema.pair_count(),
        ..Default::default()
    };

    // Two-row header. The top row carries a merged "group" band showing each
    // pair's field name once (e.g. `XUNITS` over `REF_XUNITS`/`CAN_XUNITS`); the
    // second row carries the per-column `REF`/`CAN` sub-labels. Each pair is
    // framed by a medium box (started here on the band and sub-labels, continued
    // down through the data rows below). Unpaired columns span both rows. The
    // grouping lives in the header, so it is never obscured by data highlights.
    let headers = schema.headers();
    let mut col = 0usize;
    while col < schema.column_count() {
        match schema.partner(col) {
            // Left half of an adjacent pair: merge the field-name band across
            // both columns, then label each column REF / CAN beneath it.
            Some(partner) if partner == col + 1 => {
                let field = schema.field_name(col).unwrap_or(headers[col].as_str());
                worksheet.merge_range(
                    GROUP_HEADER_ROW,
                    col as u16,
                    GROUP_HEADER_ROW,
                    (col + 1) as u16,
                    field,
                    &band_format,
                )?;
                worksheet.write_with_format(
                    LABEL_HEADER_ROW,
                    col as u16,
                    side_label(&headers[col]),
                    &pair_label_format(true),
                )?;
                worksheet.write_with_format(
                    LABEL_HEADER_ROW,
                    (col + 1) as u16,
                    side_label(&headers[col + 1]),
                    &pair_label_format(false),
                )?;
                col += 2;
            }
            // Unpaired (or a non-adjacent pair member): the full column name,
            // vertically merged across both header rows.
            _ => {
                worksheet.merge_range(
                    GROUP_HEADER_ROW,
                    col as u16,
                    LABEL_HEADER_ROW,
                    col as u16,
                    &headers[col],
                    &unpaired_header_format,
                )?;
                col += 1;
            }
        }
    }
    worksheet.set_row_height(GROUP_HEADER_ROW, HEADER_ROW_HEIGHT)?;
    worksheet.set_row_height(LABEL_HEADER_ROW, HEADER_ROW_HEIGHT)?;

    // Data rows follow the two header rows.
    let last_data_index = report.rows.len().saturating_sub(1);
    for (row_idx, record) in report.rows.iter().enumerate() {
        let excel_row = row_idx as u32 + DATA_START_ROW;
        let is_last_row = row_idx == last_data_index;
        for col in 0..schema.column_count() {
            let value = record.get(col).map(String::as_str).unwrap_or("");

            // The match-percentage column is written as a real number (so the
            // heat-map gradient and numeric sort work) with a fixed precision.
            if Some(col) == match_col {
                match value.trim().parse::<f64>() {
                    Ok(number) => {
                        worksheet.write_with_format(
                            excel_row,
                            col as u16,
                            number,
                            &percent_format,
                        )?;
                    }
                    Err(_) => {
                        worksheet.write_with_format(
                            excel_row,
                            col as u16,
                            value,
                            &percent_format,
                        )?;
                    }
                }
                continue;
            }

            // The comparison column is written as a clickable hyperlink (Excel's
            // default blue underlined link style) so users can open the deep link
            // in a browser. Blank or non-http / over-long values fall back to
            // plain text to avoid invalid links.
            if Some(col) == url_col {
                let link = value.trim();
                if (link.starts_with("http://") || link.starts_with("https://"))
                    && link.len() <= MAX_URL_LEN
                {
                    worksheet.write_url(excel_row, col as u16, Url::new(link))?;
                } else {
                    worksheet.write(excel_row, col as u16, value)?;
                }
                continue;
            }

            let state = report.cell_state(row_idx, col);
            match state {
                CellState::Match => stats.matching += 1,
                CellState::Different => stats.different += 1,
                CellState::Missing => stats.missing += 1,
                CellState::Neutral => {}
            }

            // Box edges: the pair's outer columns get medium left/right borders,
            // and the last data row closes the box with a medium bottom border.
            let (left_edge, right_edge) = pair_edges(schema, col);
            let bottom_edge = is_last_row && (left_edge || right_edge);
            match data_cell_format(state, left_edge, right_edge, bottom_edge) {
                Some(format) => {
                    worksheet.write_with_format(excel_row, col as u16, value, &format)?;
                }
                None => {
                    worksheet.write(excel_row, col as u16, value)?;
                }
            }
        }
    }
    stats.rows = report.rows.len();

    // Size every column to its content (capped) for legibility.
    for (col, width) in column_widths(report).into_iter().enumerate() {
        worksheet.set_column_width(col as u16, width)?;
    }

    // Freeze both header rows and the asset-pair identity columns — reference
    // path, candidate path, and match percentage (the leading columns up to and
    // including MATCH_PERCENTAGE) — so the pair each row describes stays visible
    // while scrolling a wide, tall report. Enable an autofilter on the second
    // (per-column) header row.
    let frozen_columns = match_col
        .map(|col| col + 1)
        .unwrap_or(1)
        .min(schema.column_count());
    worksheet.set_freeze_panes(DATA_START_ROW, frozen_columns as u16)?;
    if schema.column_count() > 0 && !report.rows.is_empty() {
        let last_col = (schema.column_count() - 1) as u16;
        let last_row = report.rows.len() as u32 + LABEL_HEADER_ROW;
        worksheet.autofilter(LABEL_HEADER_ROW, 0, last_row, last_col)?;

        // Heat-map gradient over the match-percentage column: cool at 0%, warm
        // at 50%, red-hot at 100%. Anchored to fixed 0/50/100 so the colors mean
        // the same thing regardless of the data's actual range.
        if let Some(col) = match_col {
            let gradient = ConditionalFormat3ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, 0)
                .set_midpoint(ConditionalFormatType::Number, 50)
                .set_maximum(ConditionalFormatType::Number, 100)
                .set_minimum_color(COLOR_HEAT_LOW)
                .set_midpoint_color(COLOR_HEAT_MID)
                .set_maximum_color(COLOR_HEAT_HIGH);
            worksheet.add_conditional_format(
                DATA_START_ROW,
                col as u16,
                last_row,
                col as u16,
                &gradient,
            )?;
        }
    }

    debug!(?output, "saving workbook");
    workbook.save(output)?;
    info!(
        rows = stats.rows,
        pairs = stats.pairs,
        matching = stats.matching,
        different = stats.different,
        missing = stats.missing,
        "conversion complete"
    );

    Ok(stats)
}

/// The short side label (`REF` or `CAN`) for a paired column's header.
fn side_label(header: &str) -> &'static str {
    if header.starts_with("REF_") {
        "REF"
    } else {
        "CAN"
    }
}

/// Which box edges a column sits on: `(left, right)`. The left member of an
/// adjacent pair gets a left edge; the right member a right edge. Unpaired or
/// non-adjacent columns get neither.
fn pair_edges(schema: &Schema, col: usize) -> (bool, bool) {
    match schema.partner(col) {
        Some(partner) if partner == col + 1 => (true, false),
        Some(partner) if col > 0 && partner == col - 1 => (false, true),
        _ => (false, false),
    }
}

/// The header format for a pair's `REF`/`CAN` sub-label, with a medium border on
/// the pair's outer edge (left for the `REF` column, right for the `CAN`).
fn pair_label_format(left_edge: bool) -> Format {
    let format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(COLOR_SUBHEADER_BG)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    if left_edge {
        format.set_border_left(FormatBorder::Medium)
    } else {
        format.set_border_right(FormatBorder::Medium)
    }
}

/// Builds the format for a data cell from its comparison state and which box
/// edges it sits on. Returns `None` for a plain, unhighlighted cell that is not
/// on any pair edge (written without a format, relying on default gridlines).
fn data_cell_format(state: CellState, left: bool, right: bool, bottom: bool) -> Option<Format> {
    if state == CellState::Neutral && !left && !right && !bottom {
        return None;
    }
    let mut format = Format::new();
    match state {
        CellState::Match => format = format.set_background_color(COLOR_MATCH),
        CellState::Different => format = format.set_background_color(COLOR_DIFFERENT),
        CellState::Missing => format = format.set_background_color(COLOR_MISSING),
        CellState::Neutral => {}
    }
    if left {
        format = format.set_border_left(FormatBorder::Medium);
    }
    if right {
        format = format.set_border_right(FormatBorder::Medium);
    }
    if bottom {
        format = format.set_border_bottom(FormatBorder::Medium);
    }
    Some(format)
}

/// Computes a per-column width (in characters) sized to the widest of the header
/// and any cell in that column, padded and clamped to a readable range.
///
/// The comparison-URL column is exempt from the usual width cap and sized to its
/// longest value (URLs are long but the user wants them fully visible), bounded
/// only by Excel's maximum column width.
fn column_widths(report: &Report) -> Vec<f64> {
    let column_count = report.schema.column_count();
    let url_col = report.schema.column_index(COMPARISON_URL_COLUMN);
    let mut max_chars = vec![0usize; column_count];

    for (col, header) in report.schema.headers().iter().enumerate() {
        max_chars[col] = header.chars().count();
    }
    for row in &report.rows {
        for (col, cell) in row.iter().enumerate() {
            if col < column_count {
                max_chars[col] = max_chars[col].max(cell.chars().count());
            }
        }
    }

    max_chars
        .into_iter()
        .enumerate()
        .map(|(col, chars)| {
            let width = chars as f64 + COL_WIDTH_PADDING;
            let max = if Some(col) == url_col {
                EXCEL_MAX_COL_WIDTH
            } else {
                MAX_COL_WIDTH
            };
            width.clamp(MIN_COL_WIDTH, max)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(headers: &[&str], rows: Vec<Vec<&str>>) -> Report {
        let schema = Schema::from_headers(headers.iter().map(|s| s.to_string()).collect());
        Report {
            schema,
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(String::from).collect())
                .collect(),
        }
    }

    #[test]
    fn pair_edges_mark_outer_columns_of_adjacent_pairs() {
        let s = Schema::from_headers(
            ["MATCH_PERCENTAGE", "REF_XUNITS", "CAN_XUNITS"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        );
        assert_eq!(pair_edges(&s, 0), (false, false)); // unpaired
        assert_eq!(pair_edges(&s, 1), (true, false)); // REF -> left edge
        assert_eq!(pair_edges(&s, 2), (false, true)); // CAN -> right edge
    }

    #[test]
    fn data_cell_format_is_none_only_for_plain_non_edge_cells() {
        assert!(data_cell_format(CellState::Neutral, false, false, false).is_none());
        assert!(data_cell_format(CellState::Neutral, true, false, false).is_some());
        assert!(data_cell_format(CellState::Match, false, false, false).is_some());
        assert!(data_cell_format(CellState::Different, false, false, false).is_some());
        assert!(data_cell_format(CellState::Missing, false, false, false).is_some());
        assert!(data_cell_format(CellState::Neutral, false, false, true).is_some());
    }

    #[test]
    fn column_widths_are_clamped_to_range() {
        let r = report(
            &["A", "REFERENCE_ASSET_PATH"],
            vec![vec!["x", &"y".repeat(200)]],
        );
        let widths = column_widths(&r);
        // Short column floored at the minimum; long column capped at the maximum.
        assert_eq!(widths[0], MIN_COL_WIDTH);
        assert_eq!(widths[1], MAX_COL_WIDTH);
    }

    #[test]
    fn writes_a_workbook_to_disk() {
        let r = report(
            &[
                "MATCH_PERCENTAGE",
                "REF_XUNITS",
                "CAN_XUNITS",
                "COMPARISON_URL",
            ],
            vec![
                vec!["100", "mm", "mm", "https://example.com/compare?a=1"],
                vec!["80.5", "mm", "in", "https://example.com/compare?a=2"],
                vec!["50", "mm", "", ""], // blank URL falls back to plain text
            ],
        );
        let dir = std::env::temp_dir();
        let path = dir.join("mra_xlsx_write_test.xlsx");
        let stats = write_workbook(&r, &path).expect("write should succeed");
        assert_eq!(stats.rows, 3);
        assert_eq!(stats.pairs, 1);
        assert_eq!(stats.matching, 2); // the "mm" vs "mm" pair, both cells
        assert_eq!(stats.different, 2); // the "mm" vs "in" pair, both cells
        assert_eq!(stats.missing, 2); // the "mm" vs "" pair, both cells
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }
}
