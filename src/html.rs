//! Renders a [`Graph`] into a self-contained, interactive HTML document.
//!
//! The page is a "constellation" view: a force-directed graph on a dark
//! canvas, where each node is an asset and each edge a match. All CSS and
//! JavaScript are embedded — the file has no external dependencies and works
//! offline. The graph data is serialized to JSON and spliced into the
//! template at a placeholder.

use std::path::Path;

use tracing::{debug, info};

use crate::error::AppError;
use crate::graph::{Edge, Graph, Node};
use crate::json::{push_opt_num, push_opt_str, push_str};
use crate::report::CellState;

/// The page template; the graph JSON replaces [`DATA_PLACEHOLDER`].
const TEMPLATE: &str = include_str!("html/template.html");

/// Placeholder in the template that receives the graph JSON.
const DATA_PLACEHOLDER: &str = "/*__DATA__*/null";

/// A summary of the generated document, returned for logging and reporting.
#[derive(Debug, Default, Clone, Copy)]
pub struct GraphStats {
    /// Number of unique assets (nodes).
    pub nodes: usize,
    /// Number of undirected matches (edges).
    pub edges: usize,
    /// Number of clusters (connected components with at least one edge).
    pub clusters: usize,
}

/// Writes the interactive constellation document for `graph` to `output`.
///
/// `source` is a human-readable name for the input file, shown in the page
/// header.
pub fn write_document(graph: &Graph, source: &str, output: &Path) -> Result<GraphStats, AppError> {
    let stats = GraphStats {
        nodes: graph.nodes.len(),
        edges: graph.edges.len(),
        clusters: graph.cluster_count(),
    };

    debug!(?output, "rendering HTML document");
    let document = TEMPLATE.replacen(DATA_PLACEHOLDER, &graph_json(graph, source), 1);
    std::fs::write(output, document).map_err(|source| AppError::WriteOutput {
        path: output.to_path_buf(),
        source,
    })?;

    info!(
        nodes = stats.nodes,
        edges = stats.edges,
        clusters = stats.clusters,
        "document complete"
    );
    Ok(stats)
}

/// Serializes the graph (plus the source-file name) to the JSON object the
/// template's script consumes.
fn graph_json(graph: &Graph, source: &str) -> String {
    let mut out = String::with_capacity(64 * 1024);
    out.push_str("{\"source\":");
    push_str(&mut out, source);
    out.push_str(",\"nodes\":[");
    for (i, node) in graph.nodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_node(&mut out, node);
    }
    out.push_str("],\"edges\":[");
    for (i, edge) in graph.edges.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_edge(&mut out, edge);
    }
    out.push_str("]}");
    out
}

fn push_node(out: &mut String, node: &Node) {
    out.push_str("{\"label\":");
    push_str(out, node.label());
    out.push_str(",\"path\":");
    push_str(out, &node.path);
    out.push_str(",\"uuid\":");
    push_opt_str(out, node.uuid.as_deref());
    out.push_str(",\"meta\":[");
    for (i, (field, value)) in node.meta.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        push_str(out, field);
        out.push(',');
        push_str(out, value);
        out.push(']');
    }
    out.push_str("]}");
}

fn push_edge(out: &mut String, edge: &Edge) {
    out.push_str(&format!(
        "{{\"source\":{},\"target\":{},\"geometry\":",
        edge.source, edge.target
    ));
    push_opt_num(out, edge.geometry);
    out.push_str(",\"metadata\":");
    push_opt_num(out, edge.metadata);
    out.push_str(&format!(
        ",\"comparable\":{},\"matched\":{},\"url\":",
        edge.comparable, edge.matched
    ));
    push_opt_str(out, edge.url.as_deref());
    out.push_str(",\"fields\":[");
    for (i, field) in edge.fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"f\":");
        push_str(out, &field.field);
        out.push_str(",\"r\":");
        push_str(out, &field.reference);
        out.push_str(",\"c\":");
        push_str(out, &field.candidate);
        out.push_str(&format!(
            ",\"s\":\"{}\",\"scored\":{}}}",
            state_str(field.state),
            field.scored
        ));
    }
    out.push_str("]}");
}

/// The compact state tag used in the JSON payload.
fn state_str(state: CellState) -> &'static str {
    match state {
        CellState::Match => "match",
        CellState::Different => "different",
        CellState::Missing => "missing",
        CellState::Neutral => "neutral",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{Report, Schema};

    fn graph(headers: &[&str], rows: Vec<Vec<&str>>) -> Graph {
        let schema = Schema::from_headers(headers.iter().map(|s| s.to_string()).collect());
        let report = Report {
            schema,
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(String::from).collect())
                .collect(),
        };
        Graph::from_report(&report)
    }

    #[test]
    fn graph_json_is_valid_shape() {
        let g = graph(
            &[
                "REFERENCE_ASSET_PATH",
                "CANDIDATE_ASSET_PATH",
                "MATCH_PERCENTAGE",
                "REF_XUNITS",
                "CAN_XUNITS",
            ],
            vec![vec!["a.par", "b.par", "92.5", "mm", "mm"]],
        );
        let json = graph_json(&g, "test.csv");
        assert!(json.starts_with("{\"source\":\"test.csv\""));
        assert!(json.contains("\"geometry\":92.5"));
        assert!(json.contains("\"metadata\":100"));
        assert!(json.contains("\"s\":\"match\""));
    }

    #[test]
    fn writes_a_document_to_disk() {
        let g = graph(
            &[
                "REFERENCE_ASSET_PATH",
                "CANDIDATE_ASSET_PATH",
                "MATCH_PERCENTAGE",
                "REF_XUNITS",
                "CAN_XUNITS",
            ],
            vec![
                vec!["a.par", "b.par", "92.5", "mm", "mm"],
                vec!["b.par", "c.par", "81", "mm", "in"],
            ],
        );
        let path = std::env::temp_dir().join("mra_html_write_test.html");
        let stats = write_document(&g, "test.csv", &path).expect("write should succeed");
        assert_eq!(stats.nodes, 3);
        assert_eq!(stats.edges, 2);
        assert_eq!(stats.clusters, 1);
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("\"source\":\"test.csv\""));
        assert!(!written.contains(DATA_PLACEHOLDER));
        let _ = std::fs::remove_file(&path);
    }
}
