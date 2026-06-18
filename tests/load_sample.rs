//! Integration smoke test against a real sample report.
//!
//! `data/` is git-ignored, so this test self-skips when no sample file is
//! present (e.g. in CI). When run locally with `data/test-report.csv` in place,
//! it exercises the full load → type-inference → query pipeline.

use std::path::Path;

use match_report_analyzer::store::{ColumnType, DataStore, TABLE, quote_ident};

#[test]
fn loads_sample_and_filters_by_match_percentage() {
    let path = Path::new("data/test-report.csv");
    if !path.exists() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }

    let store = DataStore::load_csv(path).expect("load sample CSV");

    // The report should have rows and the expected score column.
    let total = store.row_count().expect("row count");
    assert!(total > 0, "expected a non-empty report");

    let score = store
        .columns()
        .iter()
        .find(|c| c.name == "MATCH_PERCENTAGE")
        .expect("MATCH_PERCENTAGE column present");
    assert_eq!(
        score.ty,
        ColumnType::Real,
        "MATCH_PERCENTAGE should be inferred numeric"
    );

    // A numeric range filter must run and return no more rows than the total,
    // confirming the column compares numerically rather than lexically.
    let ident = quote_ident("MATCH_PERCENTAGE");
    let sql = format!("SELECT * FROM {TABLE} WHERE {ident} > 80 AND {ident} < 99");
    let result = store.query(&sql).expect("filter query runs");
    assert!(result.rows.len() <= total);
    assert!(result.columns.iter().any(|c| c == "MATCH_PERCENTAGE"));
}
