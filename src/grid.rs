//! Renders a [`Report`] into a self-contained, interactive data-grid HTML
//! document.
//!
//! The grid mirrors the Excel view — grouped `REF_`/`CAN_` pair headers,
//! per-cell match/difference/missing highlighting, a heat-mapped match
//! percentage, and clickable comparison links — as a single page. On top of
//! that it adds live exploration: search by asset name/path/UUID, sorting by
//! any column, a SQL-like `WHERE` filter combining rules across columns, and
//! in-place cell editing for "what-if" scenarios (edits re-classify and
//! restyle the pair instantly and can be reset).
//!
//! All CSS and JavaScript are embedded — the file has no external dependencies
//! and works offline. The report data is serialized to JSON and spliced into
//! the template at a placeholder; the comparison logic in the page mirrors
//! [`crate::report::classify`] so live edits recolor exactly like the static
//! renderings.

use std::path::Path;

use tracing::{debug, info};

use crate::error::AppError;
use crate::json::{push_opt_num, push_str};
use crate::report::{
    CANDIDATE_ASSET_PATH_COLUMN, COMPARISON_URL_COLUMN, MATCH_PERCENTAGE_COLUMN, REF_PREFIX,
    REFERENCE_ASSET_PATH_COLUMN, Report,
};

/// The page template; the report JSON replaces [`DATA_PLACEHOLDER`].
const TEMPLATE: &str = include_str!("html/grid_template.html");

/// Placeholder in the template that receives the report JSON.
const DATA_PLACEHOLDER: &str = "/*__DATA__*/null";

/// Metadata fields whose columns are searched (in addition to the two asset
/// path columns) by the grid's quick-search box: asset names and UUIDs.
const SEARCH_FIELDS: [&str; 2] = ["XNAME", "XID"];

/// A summary of the generated document, returned for logging and reporting.
#[derive(Debug, Default, Clone, Copy)]
pub struct GridStats {
    /// Number of data rows written.
    pub rows: usize,
    /// Number of columns in the report.
    pub columns: usize,
    /// Number of comparable `REF_`/`CAN_` pairs in the schema.
    pub pairs: usize,
}

/// Writes the interactive data-grid document for `report` to `output`.
///
/// `source` is a human-readable name for the input file, shown in the page
/// header.
pub fn write_document(report: &Report, source: &str, output: &Path) -> Result<GridStats, AppError> {
    let stats = GridStats {
        rows: report.rows.len(),
        columns: report.schema.column_count(),
        pairs: report.schema.pair_count(),
    };

    debug!(?output, "rendering grid document");
    let document = TEMPLATE.replacen(DATA_PLACEHOLDER, &grid_json(report, source), 1);
    std::fs::write(output, document).map_err(|source| AppError::WriteOutput {
        path: output.to_path_buf(),
        source,
    })?;

    info!(
        rows = stats.rows,
        columns = stats.columns,
        pairs = stats.pairs,
        "document complete"
    );
    Ok(stats)
}

/// Serializes the report (plus the source-file name) to the JSON object the
/// template's script consumes.
fn grid_json(report: &Report, source: &str) -> String {
    let schema = &report.schema;
    let mut out = String::with_capacity(64 * 1024);

    out.push_str("{\"source\":");
    push_str(&mut out, source);

    // Special columns the page treats differently (may be absent).
    out.push_str(",\"matchCol\":");
    push_opt_num(
        &mut out,
        schema
            .column_index(MATCH_PERCENTAGE_COLUMN)
            .map(|i| i as f64),
    );
    out.push_str(",\"urlCol\":");
    push_opt_num(
        &mut out,
        schema.column_index(COMPARISON_URL_COLUMN).map(|i| i as f64),
    );

    // Columns the quick-search box scans: asset paths, names, and UUIDs.
    let search_cols: Vec<usize> = (0..schema.column_count())
        .filter(|&col| {
            let header = &schema.headers()[col];
            header == REFERENCE_ASSET_PATH_COLUMN
                || header == CANDIDATE_ASSET_PATH_COLUMN
                || schema
                    .field_name(col)
                    .is_some_and(|field| SEARCH_FIELDS.contains(&field))
        })
        .collect();
    out.push_str(",\"searchCols\":[");
    for (i, col) in search_cols.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&col.to_string());
    }
    out.push(']');

    // The column structure: name, plus pair info for REF_/CAN_ columns.
    out.push_str(",\"columns\":[");
    for col in 0..schema.column_count() {
        if col > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":");
        push_str(&mut out, &schema.headers()[col]);
        match schema.partner(col) {
            Some(partner) => {
                out.push_str(",\"field\":");
                push_str(&mut out, schema.field_name(col).unwrap_or(""));
                out.push_str(&format!(
                    ",\"side\":\"{}\",\"partner\":{partner}}}",
                    if schema.headers()[col].starts_with(REF_PREFIX) {
                        "ref"
                    } else {
                        "can"
                    }
                ));
            }
            None => out.push_str(",\"field\":null,\"side\":null,\"partner\":null}"),
        }
    }
    out.push(']');

    // The data rows, aligned (padded/truncated) to the column count so the
    // page never has to bounds-check ragged rows.
    out.push_str(",\"rows\":[");
    for (i, row) in report.rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        for col in 0..schema.column_count() {
            if col > 0 {
                out.push(',');
            }
            push_str(&mut out, row.get(col).map(String::as_str).unwrap_or(""));
        }
        out.push(']');
    }
    out.push_str("]}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Schema;

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

    const HEADERS: [&str; 6] = [
        "REFERENCE_ASSET_PATH",
        "CANDIDATE_ASSET_PATH",
        "MATCH_PERCENTAGE",
        "REF_XUNITS",
        "CAN_XUNITS",
        "COMPARISON_URL",
    ];

    #[test]
    fn grid_json_describes_columns_and_pairs() {
        let r = report(
            &HEADERS,
            vec![vec!["a", "b", "92.5", "mm", "mm", "https://x.example/1"]],
        );
        let json = grid_json(&r, "test.csv");
        assert!(json.starts_with("{\"source\":\"test.csv\""));
        assert!(json.contains("\"matchCol\":2"));
        assert!(json.contains("\"urlCol\":5"));
        assert!(json.contains("\"searchCols\":[0,1]"));
        assert!(json.contains(
            "{\"name\":\"REF_XUNITS\",\"field\":\"XUNITS\",\"side\":\"ref\",\"partner\":4}"
        ));
        assert!(json.contains(
            "{\"name\":\"CAN_XUNITS\",\"field\":\"XUNITS\",\"side\":\"can\",\"partner\":3}"
        ));
        assert!(json.contains("\"rows\":[[\"a\",\"b\",\"92.5\",\"mm\",\"mm\","));
    }

    #[test]
    fn search_columns_include_name_and_uuid_pairs() {
        let r = report(
            &[
                "REFERENCE_ASSET_PATH",
                "CANDIDATE_ASSET_PATH",
                "MATCH_PERCENTAGE",
                "REF_XID",
                "CAN_XID",
                "REF_XNAME",
                "CAN_XNAME",
                "REF_XUNITS",
                "CAN_XUNITS",
            ],
            vec![],
        );
        let json = grid_json(&r, "test.csv");
        assert!(json.contains("\"searchCols\":[0,1,3,4,5,6]"));
    }

    #[test]
    fn ragged_rows_are_padded_to_the_column_count() {
        let r = report(&HEADERS, vec![vec!["a", "b"]]);
        let json = grid_json(&r, "test.csv");
        assert!(json.contains("\"rows\":[[\"a\",\"b\",\"\",\"\",\"\",\"\"]]"));
    }

    #[test]
    fn writes_a_document_to_disk() {
        let r = report(
            &HEADERS,
            vec![
                vec!["a", "b", "92.5", "mm", "mm", ""],
                vec!["c", "d", "81", "mm", "in", ""],
            ],
        );
        let path = std::env::temp_dir().join("mra_grid_write_test.html");
        let stats = write_document(&r, "test.csv", &path).expect("write should succeed");
        assert_eq!(stats.rows, 2);
        assert_eq!(stats.columns, 6);
        assert_eq!(stats.pairs, 1);
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("\"source\":\"test.csv\""));
        assert!(!written.contains(DATA_PLACEHOLDER));
        let _ = std::fs::remove_file(&path);
    }
}
