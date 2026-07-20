//! Integration tests.
//!
//! Tests marked "sample report" exercise the real Physna export at
//! `data/test-report.csv`. That directory's contents are git-ignored (see
//! `data/README.md`), so those tests skip — with a note — when the file is
//! absent (e.g. in CI or a fresh clone); the remaining tests are
//! self-contained. The expected numbers are properties of that specific
//! sample file; regenerate them with `cargo run -- <cmd> data/test-report.csv …`
//! if the sample is ever replaced.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use match_report_analyzer::graph::Graph;
use match_report_analyzer::report::Report;
use match_report_analyzer::{AppError, convert_to_graph, convert_to_grid, convert_to_xlsx};

/// The compiled binary under test.
const BIN: &str = env!("CARGO_BIN_EXE_match-report-analyzer");

/// The sample match report, when the developer has provided one.
fn sample_csv() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/test-report.csv");
    if path.exists() {
        Some(path)
    } else {
        eprintln!(
            "skipping: {} not present (data/ contents are git-ignored; see data/README.md)",
            path.display()
        );
        None
    }
}

/// A per-process unique temp path, cleaned up by [`TempFile`]'s `Drop`.
struct TempFile(PathBuf);

impl TempFile {
    fn new(name: &str) -> Self {
        TempFile(std::env::temp_dir().join(format!("mra_it_{}_{name}", std::process::id())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ---- library API on the sample report --------------------------------------

#[test]
fn xlsx_stats_on_sample_report() {
    let Some(input) = sample_csv() else { return };
    let output = TempFile::new("sample.xlsx");

    let stats = convert_to_xlsx(&input, output.path()).expect("conversion should succeed");

    assert_eq!(stats.rows, 133);
    assert_eq!(stats.pairs, 12);
    assert_eq!(stats.matching, 196);
    assert_eq!(stats.different, 128);
    assert_eq!(stats.missing, 1992);
    // Every highlight colors both halves of a pair, so the counts are even.
    assert_eq!(stats.matching % 2, 0);
    assert_eq!(stats.different % 2, 0);
    assert_eq!(stats.missing % 2, 0);
    assert!(output.path().metadata().expect("file written").len() > 0);
}

#[test]
fn graph_stats_on_sample_report() {
    let Some(input) = sample_csv() else { return };
    let output = TempFile::new("sample.html");

    let stats = convert_to_graph(&input, output.path()).expect("conversion should succeed");

    assert_eq!(stats.nodes, 143);
    assert_eq!(stats.edges, 133);
    assert_eq!(stats.clusters, 58);

    let document = std::fs::read_to_string(output.path()).expect("file written");
    assert!(document.contains("\"source\":\"test-report.csv\""));
    assert!(document.contains("\"nodes\":["));
    // The data placeholder must have been replaced by the payload.
    assert!(!document.contains("/*__DATA__*/null"));
    // The payload (a single line, since control characters are escaped) must
    // not be able to terminate the surrounding <script> element.
    let payload = document
        .lines()
        .find(|line| line.contains("const DATA ="))
        .expect("payload line");
    assert!(!payload.contains("</"), "JSON payload must escape '<'");
}

#[test]
fn grid_stats_on_sample_report() {
    let Some(input) = sample_csv() else { return };
    let output = TempFile::new("sample-grid.html");

    let stats = convert_to_grid(&input, output.path()).expect("conversion should succeed");

    assert_eq!(stats.rows, 133);
    assert_eq!(stats.columns, 30);
    assert_eq!(stats.pairs, 12);

    let document = std::fs::read_to_string(output.path()).expect("file written");
    assert!(document.contains("\"source\":\"test-report.csv\""));
    assert!(document.contains("\"matchCol\":2"));
    assert!(document.contains("\"searchCols\":"));
    assert!(!document.contains("/*__DATA__*/null"));
    // The single-line payload must not be able to terminate the <script>.
    let payload = document
        .lines()
        .find(|line| line.contains("const DATA ="))
        .expect("payload line");
    assert!(!payload.contains("</"), "JSON payload must escape '<'");

    // The grid is pre-sorted like the Excel view: match percentage descending.
    let rows_json = payload.split_once("\"rows\":[").expect("rows array").1;
    let first_row_pct: f64 = rows_json
        .split('"')
        .nth(5) // third cell of the first row: ["path","path","<pct>",…]
        .expect("match percentage cell")
        .parse()
        .expect("numeric match percentage");
    assert_eq!(first_row_pct, 100.0);
}

#[test]
fn graph_invariants_on_sample_report() {
    let Some(input) = sample_csv() else { return };
    let report = Report::from_csv_path(&input).expect("sample parses");
    let graph = Graph::from_report(&report);

    assert_eq!(graph.nodes.len(), 143);
    assert_eq!(graph.edges.len(), 133);

    // Node identities (UUID, else path) are unique after deduplication.
    let keys: BTreeSet<&str> = graph
        .nodes
        .iter()
        .map(|n| n.uuid.as_deref().unwrap_or(n.path.as_str()))
        .collect();
    assert_eq!(keys.len(), graph.nodes.len());
    assert!(graph.nodes.iter().all(|n| !n.path.is_empty()));
    assert!(graph.nodes.iter().all(|n| !n.label().is_empty()));

    let mut seen_pairs = BTreeSet::new();
    for edge in &graph.edges {
        // Structural sanity: in-bounds, no self-loops, no duplicate pairs.
        assert!(edge.source < graph.nodes.len());
        assert!(edge.target < graph.nodes.len());
        assert_ne!(edge.source, edge.target);
        let key = (edge.source.min(edge.target), edge.source.max(edge.target));
        assert!(seen_pairs.insert(key), "duplicate undirected edge {key:?}");

        // Score sanity: percentages in range, counts consistent.
        if let Some(geometry) = edge.geometry {
            assert!((0.0..=100.0).contains(&geometry));
        }
        assert!(edge.matched <= edge.comparable);
        match edge.metadata {
            Some(metadata) => {
                assert!(edge.comparable > 0);
                assert!((0.0..=100.0).contains(&metadata));
            }
            None => assert_eq!(edge.comparable, 0),
        }
    }
}

// ---- binary end-to-end on the sample report --------------------------------

#[test]
fn binary_converts_sample_to_xlsx() {
    let Some(input) = sample_csv() else { return };
    let output = TempFile::new("bin.xlsx");

    let run = Command::new(BIN)
        .args([
            "xlsx",
            input.to_str().unwrap(),
            output.path().to_str().unwrap(),
        ])
        .output()
        .expect("binary runs");

    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("Wrote"), "stdout: {stdout}");
    assert!(stdout.contains("133 rows"), "stdout: {stdout}");
    assert!(output.path().exists());
}

#[test]
fn binary_converts_sample_to_graph_normalizing_extension() {
    let Some(input) = sample_csv() else { return };
    // Ask for `.htm` via the legacy `html` alias; the tool must accept the
    // alias and correct the extension to `.html`.
    let requested = TempFile::new("bin.htm");
    let corrected = TempFile::new("bin.html");

    let run = Command::new(BIN)
        .args([
            "html",
            input.to_str().unwrap(),
            requested.path().to_str().unwrap(),
        ])
        .output()
        .expect("binary runs");

    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("143 assets, 133 matches, 58 clusters"),
        "stdout: {stdout}"
    );
    assert!(
        corrected.path().exists(),
        "output written under corrected extension"
    );
    assert!(
        !requested.path().exists(),
        "no file under the requested extension"
    );
}

#[test]
fn binary_converts_sample_to_grid() {
    let Some(input) = sample_csv() else { return };
    let output = TempFile::new("bin-grid.html");

    let run = Command::new(BIN)
        .args([
            "grid",
            input.to_str().unwrap(),
            output.path().to_str().unwrap(),
        ])
        .output()
        .expect("binary runs");

    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("133 rows, 30 columns, 12 pairs"),
        "stdout: {stdout}"
    );
    assert!(output.path().exists());
}

#[test]
fn binary_fails_cleanly_for_missing_input() {
    let run = Command::new(BIN)
        .args(["xlsx", "no-such-file.csv", "out.xlsx"])
        .output()
        .expect("binary runs");
    assert!(!run.status.success());
    assert!(!Path::new("out.xlsx").exists());
}

// ---- self-contained validation paths ---------------------------------------

#[test]
fn rejects_input_without_csv_extension() {
    let output = TempFile::new("rejected.xlsx");
    let err = convert_to_xlsx(Path::new("report.txt"), output.path())
        .expect_err("non-.csv input must be rejected");
    assert!(matches!(err, AppError::NotCsv { .. }), "got: {err}");
}

#[test]
fn rejects_report_missing_required_columns() {
    let input = TempFile::new("no-required.csv");
    std::fs::write(input.path(), "A,B\n1,2\n").unwrap();
    let output = TempFile::new("no-required.xlsx");

    let err = convert_to_xlsx(input.path(), output.path())
        .expect_err("report without required columns must be rejected");
    let AppError::MissingRequiredColumns { columns } = err else {
        panic!("got: {err}");
    };
    assert_eq!(
        columns,
        vec![
            "REFERENCE_ASSET_PATH",
            "CANDIDATE_ASSET_PATH",
            "MATCH_PERCENTAGE"
        ]
    );
}

#[test]
fn converts_report_with_headers_but_no_rows() {
    let input = TempFile::new("empty.csv");
    std::fs::write(
        input.path(),
        "REFERENCE_ASSET_PATH,CANDIDATE_ASSET_PATH,MATCH_PERCENTAGE\n",
    )
    .unwrap();

    let xlsx_out = TempFile::new("empty.xlsx");
    let stats = convert_to_xlsx(input.path(), xlsx_out.path()).expect("empty report converts");
    assert_eq!(stats.rows, 0);

    let graph_out = TempFile::new("empty.html");
    let stats = convert_to_graph(input.path(), graph_out.path()).expect("empty report converts");
    assert_eq!(stats.nodes, 0);
    assert_eq!(stats.edges, 0);
    assert_eq!(stats.clusters, 0);

    let grid_out = TempFile::new("empty-grid.html");
    let stats = convert_to_grid(input.path(), grid_out.path()).expect("empty report converts");
    assert_eq!(stats.rows, 0);
    assert_eq!(stats.columns, 3);
    assert_eq!(stats.pairs, 0);
}
