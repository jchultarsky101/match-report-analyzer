//! In-memory model of a match-report CSV.
//!
//! A match report pairs the metadata of a *reference* asset against a
//! *candidate* asset. Paired metadata columns are named `REF_<field>` and
//! `CAN_<field>`, where `<field>` is the actual metadata field name and must be
//! identical between the two columns. All other columns (e.g.
//! `REFERENCE_ASSET_PATH`, `MATCH_PERCENTAGE`, `COMPARISON_URL`) are plain,
//! unpaired columns.

use std::path::Path;

use crate::error::AppError;

/// Prefix marking a column that holds a *reference* asset's metadata value.
const REF_PREFIX: &str = "REF_";
/// Prefix marking a column that holds a *candidate* asset's metadata value.
const CAN_PREFIX: &str = "CAN_";

/// The comparison state of a single `REF_`/`CAN_` cell pair within a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    /// The reference and candidate values are equal (after trimming), or the
    /// cell does not participate in a comparable pair.
    Equal,
    /// Both values are present but differ.
    Different,
    /// Exactly one of the two values is empty (missing on one side).
    Missing,
}

/// Compares a reference value against a candidate value.
///
/// Comparison is performed on the trimmed string representation so that the
/// CSV's exact textual values are preserved and never silently coerced. Two
/// empty values are considered [`CellState::Equal`].
pub fn classify(reference: &str, candidate: &str) -> CellState {
    let reference = reference.trim();
    let candidate = candidate.trim();

    if reference == candidate {
        CellState::Equal
    } else if reference.is_empty() || candidate.is_empty() {
        CellState::Missing
    } else {
        CellState::Different
    }
}

/// The structure of a match report: its header row plus, for every column, the
/// index of its partner column if it participates in a `REF_`/`CAN_` pair.
#[derive(Debug, Clone)]
pub struct Schema {
    /// The header labels, in column order.
    headers: Vec<String>,
    /// For each column index, `Some(partner_index)` if the column is one half of
    /// a `REF_`/`CAN_` pair, otherwise `None`.
    partners: Vec<Option<usize>>,
}

impl Schema {
    /// Builds a [`Schema`] from a header row by pairing `REF_<field>` columns
    /// with their `CAN_<field>` counterparts.
    fn from_headers(headers: Vec<String>) -> Self {
        // Map each metadata field name to the column index of its REF_ and CAN_
        // halves (whichever are present).
        let mut ref_cols: Vec<(String, usize)> = Vec::new();
        let mut can_cols: Vec<(String, usize)> = Vec::new();

        for (idx, header) in headers.iter().enumerate() {
            if let Some(field) = header.strip_prefix(REF_PREFIX) {
                ref_cols.push((field.to_string(), idx));
            } else if let Some(field) = header.strip_prefix(CAN_PREFIX) {
                can_cols.push((field.to_string(), idx));
            }
        }

        let mut partners = vec![None; headers.len()];
        for (field, ref_idx) in &ref_cols {
            if let Some((_, can_idx)) = can_cols.iter().find(|(f, _)| f == field) {
                partners[*ref_idx] = Some(*can_idx);
                partners[*can_idx] = Some(*ref_idx);
            }
        }

        Schema { headers, partners }
    }

    /// The header labels, in column order.
    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    /// The number of columns.
    pub fn column_count(&self) -> usize {
        self.headers.len()
    }

    /// The partner column index for `column`, if it participates in a pair.
    pub fn partner(&self, column: usize) -> Option<usize> {
        self.partners.get(column).copied().flatten()
    }

    /// The number of comparable `REF_`/`CAN_` pairs in the schema.
    pub fn pair_count(&self) -> usize {
        // Each pair is counted twice (once per half), so divide by two.
        self.partners.iter().filter(|p| p.is_some()).count() / 2
    }
}

/// A fully-parsed match report: its [`Schema`] and all data rows.
#[derive(Debug, Clone)]
pub struct Report {
    /// The column structure.
    pub schema: Schema,
    /// The data rows, each a vector of cell values aligned to the schema's
    /// columns.
    pub rows: Vec<Vec<String>>,
}

impl Report {
    /// Reads and parses a match-report CSV from `path`.
    pub fn from_csv_path(path: &Path) -> Result<Self, AppError> {
        let file = std::fs::File::open(path).map_err(|source| AppError::ReadInput {
            path: path.to_path_buf(),
            source,
        })?;

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(file);

        let headers: Vec<String> = reader.headers()?.iter().map(|h| h.to_string()).collect();

        if headers.is_empty() {
            return Err(AppError::MissingHeaders);
        }

        let schema = Schema::from_headers(headers);

        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record?;
            let row: Vec<String> = record.iter().map(|f| f.to_string()).collect();
            rows.push(row);
        }

        Ok(Report { schema, rows })
    }

    /// Classifies the cell at `(row, column)` against its pair partner.
    ///
    /// Returns [`CellState::Equal`] for columns that are not part of a pair, or
    /// when either cell is out of bounds (e.g. a short, ragged row).
    pub fn cell_state(&self, row: usize, column: usize) -> CellState {
        let Some(partner) = self.schema.partner(column) else {
            return CellState::Equal;
        };
        let Some(record) = self.rows.get(row) else {
            return CellState::Equal;
        };
        let here = record.get(column).map(String::as_str).unwrap_or("");
        let there = record.get(partner).map(String::as_str).unwrap_or("");

        // Determine which side is the reference so the semantics of `classify`
        // (missing-on-one-side) are stable regardless of column order.
        let is_reference = self.schema.headers()[column].starts_with(REF_PREFIX);
        if is_reference {
            classify(here, there)
        } else {
            classify(there, here)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(headers: &[&str]) -> Schema {
        Schema::from_headers(headers.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn classify_equal_values() {
        assert_eq!(classify("mm", "mm"), CellState::Equal);
        assert_eq!(classify(" mm ", "mm"), CellState::Equal);
        assert_eq!(classify("", ""), CellState::Equal);
    }

    #[test]
    fn classify_different_values() {
        assert_eq!(classify("true", "false"), CellState::Different);
        assert_eq!(classify("160.00", "341.50"), CellState::Different);
    }

    #[test]
    fn classify_missing_on_one_side() {
        assert_eq!(classify("mm", ""), CellState::Missing);
        assert_eq!(classify("", "mm"), CellState::Missing);
        assert_eq!(classify("  ", "mm"), CellState::Missing);
    }

    #[test]
    fn pairs_matching_ref_and_can_columns() {
        let s = schema(&[
            "REFERENCE_ASSET_PATH",
            "REF_XUNITS",
            "CAN_XUNITS",
            "COMPARISON_URL",
        ]);
        assert_eq!(s.pair_count(), 1);
        assert_eq!(s.partner(1), Some(2));
        assert_eq!(s.partner(2), Some(1));
        assert_eq!(s.partner(0), None);
        assert_eq!(s.partner(3), None);
    }

    #[test]
    fn pairs_handle_double_underscore_field_names() {
        // REF__COST_($) / CAN__COST_($): field name is "_COST_($)".
        let s = schema(&["REF__COST_($)", "CAN__COST_($)"]);
        assert_eq!(s.pair_count(), 1);
        assert_eq!(s.partner(0), Some(1));
    }

    #[test]
    fn unpaired_prefixed_columns_have_no_partner() {
        // A REF_ column with no matching CAN_ column is left unpaired.
        let s = schema(&["REF_ONLY", "CAN_OTHER"]);
        assert_eq!(s.pair_count(), 0);
        assert_eq!(s.partner(0), None);
        assert_eq!(s.partner(1), None);
    }

    #[test]
    fn reference_lookalike_columns_are_not_treated_as_prefixed() {
        // "REFERENCE_..." and "CANDIDATE_..." must not match the REF_/CAN_ prefixes.
        let s = schema(&["REFERENCE_ASSET_PATH", "CANDIDATE_ASSET_PATH"]);
        assert_eq!(s.pair_count(), 0);
    }

    #[test]
    fn cell_state_reads_from_rows() {
        let s = schema(&["REF_XUNITS", "CAN_XUNITS"]);
        let report = Report {
            schema: s,
            rows: vec![
                vec!["mm".into(), "mm".into()],
                vec!["mm".into(), "in".into()],
                vec!["mm".into(), "".into()],
            ],
        };
        assert_eq!(report.cell_state(0, 0), CellState::Equal);
        assert_eq!(report.cell_state(1, 0), CellState::Different);
        assert_eq!(report.cell_state(1, 1), CellState::Different);
        assert_eq!(report.cell_state(2, 0), CellState::Missing);
        assert_eq!(report.cell_state(2, 1), CellState::Missing);
    }
}
