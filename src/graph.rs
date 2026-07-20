//! Builds an asset-similarity graph from a match report.
//!
//! Each row of a match report is one *match* between a reference asset and a
//! candidate asset, but the same asset can appear in many rows and on either
//! side. Taken together the rows therefore describe a graph: assets are nodes
//! and matches are edges. This module deduplicates assets into nodes (keyed by
//! their UUID when one is known, otherwise by their path), merges duplicate or
//! reversed matches into single undirected edges, and scores each edge on two
//! axes:
//!
//! - **geometry** — the report's `MATCH_PERCENTAGE` (0–100), and
//! - **metadata** — the percentage of *intrinsic* `REF_`/`CAN_` field pairs
//!   (present on both sides) whose values agree.
//!
//! Identity and organizational fields (UUID, folder, owner, name) are carried
//! along for display but excluded from the metadata score: two distinct assets
//! always differ on identity, and folder/owner say where an asset lives, not
//! what it is.

use std::collections::{BTreeMap, HashMap};

use crate::report::{
    CANDIDATE_ASSET_PATH_COLUMN, COMPARISON_URL_COLUMN, CellState, MATCH_PERCENTAGE_COLUMN,
    REF_PREFIX, REFERENCE_ASSET_PATH_COLUMN, Report, classify,
};

/// Metadata field holding an asset's UUID. Used as node identity (falling back
/// to the asset path when absent), never scored.
pub const UUID_FIELD: &str = "XID";

/// Metadata field holding an asset's display name, used for node labels.
const NAME_FIELD: &str = "XNAME";

/// Fields that identify or locate an asset rather than describe it. They are
/// shown in the details panel but excluded from the metadata-similarity score.
pub const UNSCORED_FIELDS: [&str; 5] = [
    UUID_FIELD,
    "XFOLDER_ID",
    "XFOLDER_NAME",
    NAME_FIELD,
    "XOWNER_ID",
];

/// A unique asset appearing in the report, on either side of any match.
#[derive(Debug, Clone)]
pub struct Node {
    /// The asset's path as it appears in the report.
    pub path: String,
    /// The asset's UUID, when any row provided one for this path.
    pub uuid: Option<String>,
    /// Every metadata field/value known for this asset, in first-seen order,
    /// collected across all rows that mention it (values are trimmed;
    /// empty values are never stored).
    pub meta: Vec<(String, String)>,
}

impl Node {
    /// The display label: the asset's name field when known, otherwise the
    /// final path segment.
    pub fn label(&self) -> &str {
        self.meta
            .iter()
            .find(|(field, _)| field == NAME_FIELD)
            .map(|(_, value)| value.as_str())
            .unwrap_or_else(|| self.path.rsplit('/').next().unwrap_or(&self.path))
    }
}

/// One `REF_`/`CAN_` field pair of a match, compared for the details view.
#[derive(Debug, Clone)]
pub struct FieldComparison {
    /// The metadata field name (without the `REF_`/`CAN_` prefix).
    pub field: String,
    /// The reference asset's trimmed value.
    pub reference: String,
    /// The candidate asset's trimmed value.
    pub candidate: String,
    /// How the two values compare.
    pub state: CellState,
    /// Whether this field counts toward the metadata score (intrinsic fields
    /// only; identity/organizational fields are `false`).
    pub scored: bool,
}

/// An undirected match between two assets.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Index into [`Graph::nodes`] of the *reference* asset of the kept row.
    pub source: usize,
    /// Index into [`Graph::nodes`] of the *candidate* asset of the kept row.
    pub target: usize,
    /// The geometric `MATCH_PERCENTAGE` (0–100), when it parses as a number.
    pub geometry: Option<f64>,
    /// Percentage (0–100) of comparable intrinsic fields whose values agree,
    /// or `None` when no intrinsic field is present on both sides.
    pub metadata: Option<f64>,
    /// Number of intrinsic fields present on both sides.
    pub comparable: usize,
    /// Number of those fields whose values agree.
    pub matched: usize,
    /// Field-by-field comparison, for every pair with a value on either side.
    pub fields: Vec<FieldComparison>,
    /// The deep-link comparison URL, when present.
    pub url: Option<String>,
}

/// The similarity graph built from a report: deduplicated assets plus merged,
/// scored matches.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    /// The unique assets.
    pub nodes: Vec<Node>,
    /// The undirected matches between them.
    pub edges: Vec<Edge>,
}

impl Graph {
    /// Builds the similarity graph from a parsed report.
    ///
    /// Rows missing either asset path, and self-matches (both sides resolving
    /// to the same asset), are skipped. When the same pair of assets appears in
    /// several rows (including reversed), the row with the most comparable
    /// metadata wins (ties broken by higher geometry score).
    pub fn from_report(report: &Report) -> Self {
        let schema = &report.schema;
        let Some(ref_path_col) = schema.column_index(REFERENCE_ASSET_PATH_COLUMN) else {
            return Graph::default();
        };
        let Some(can_path_col) = schema.column_index(CANDIDATE_ASSET_PATH_COLUMN) else {
            return Graph::default();
        };
        let match_col = schema.column_index(MATCH_PERCENTAGE_COLUMN);
        let url_col = schema.column_index(COMPARISON_URL_COLUMN);

        // The REF_/CAN_ pairs as (field name, ref column, can column).
        let pairs: Vec<(String, usize, usize)> = (0..schema.column_count())
            .filter_map(|col| {
                let partner = schema.partner(col)?;
                let field = schema.field_name(col)?;
                schema.headers()[col]
                    .starts_with(REF_PREFIX)
                    .then(|| (field.to_string(), col, partner))
            })
            .collect();
        let uuid_pair = pairs.iter().find(|(field, _, _)| field == UUID_FIELD);

        /// The trimmed value of `col` in `row` (empty for out-of-bounds cells).
        fn cell(row: &[String], col: usize) -> &str {
            row.get(col).map(String::as_str).unwrap_or("").trim()
        }

        // First pass: learn each path's UUID from any row that provides one, so
        // an asset appearing later without metadata still lands on the same
        // node.
        let mut path_uuid: HashMap<String, String> = HashMap::new();
        if let Some((_, ref_uuid_col, can_uuid_col)) = uuid_pair {
            for row in &report.rows {
                for (path_col, uuid_col) in
                    [(ref_path_col, *ref_uuid_col), (can_path_col, *can_uuid_col)]
                {
                    let path = cell(row, path_col);
                    let uuid = cell(row, uuid_col);
                    if !path.is_empty() && !uuid.is_empty() {
                        path_uuid
                            .entry(path.to_string())
                            .or_insert(uuid.to_string());
                    }
                }
            }
        }
        let key_of = |path: &str| {
            path_uuid
                .get(path)
                .cloned()
                .unwrap_or_else(|| path.to_string())
        };

        // Second pass: create nodes on first sight, merge metadata, and build
        // deduplicated undirected edges. BTreeMap keeps the output order (and
        // therefore the generated file) deterministic.
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut nodes: Vec<Node> = Vec::new();
        let mut edge_map: BTreeMap<(usize, usize), Edge> = BTreeMap::new();

        for row in &report.rows {
            let ref_path = cell(row, ref_path_col);
            let can_path = cell(row, can_path_col);
            if ref_path.is_empty() || can_path.is_empty() || key_of(ref_path) == key_of(can_path) {
                continue;
            }

            let mut ensure = |path: &str| -> usize {
                *index.entry(key_of(path)).or_insert_with(|| {
                    nodes.push(Node {
                        path: path.to_string(),
                        uuid: path_uuid.get(path).cloned(),
                        meta: Vec::new(),
                    });
                    nodes.len() - 1
                })
            };
            let source = ensure(ref_path);
            let target = ensure(can_path);

            // Merge each side's non-empty values into its node's metadata.
            for (field, ref_col, can_col) in &pairs {
                for (node, col) in [(source, *ref_col), (target, *can_col)] {
                    let value = cell(row, col);
                    if !value.is_empty() && !nodes[node].meta.iter().any(|(f, _)| f == field) {
                        nodes[node].meta.push((field.clone(), value.to_string()));
                    }
                }
            }

            // Compare every field pair and score the intrinsic ones.
            let mut fields = Vec::new();
            let mut comparable = 0usize;
            let mut matched = 0usize;
            for (field, ref_col, can_col) in &pairs {
                let reference = cell(row, *ref_col);
                let candidate = cell(row, *can_col);
                if reference.is_empty() && candidate.is_empty() {
                    continue;
                }
                let state = classify(reference, candidate);
                let scored = !UNSCORED_FIELDS.contains(&field.as_str());
                if scored && !reference.is_empty() && !candidate.is_empty() {
                    comparable += 1;
                    if state == CellState::Match {
                        matched += 1;
                    }
                }
                fields.push(FieldComparison {
                    field: field.clone(),
                    reference: reference.to_string(),
                    candidate: candidate.to_string(),
                    state,
                    scored,
                });
            }

            let edge = Edge {
                source,
                target,
                geometry: match_col
                    .and_then(|col| cell(row, col).parse::<f64>().ok())
                    .filter(|value| value.is_finite()),
                metadata: (comparable > 0).then(|| matched as f64 / comparable as f64 * 100.0),
                comparable,
                matched,
                fields,
                url: url_col
                    .map(|col| cell(row, col))
                    .filter(|value| !value.is_empty())
                    .map(String::from),
            };

            // Undirected dedup: the row with more comparable metadata wins,
            // ties broken by higher geometry score.
            let key = (source.min(target), source.max(target));
            match edge_map.get(&key) {
                Some(existing)
                    if (existing.comparable, existing.geometry.unwrap_or(-1.0))
                        >= (edge.comparable, edge.geometry.unwrap_or(-1.0)) => {}
                _ => {
                    edge_map.insert(key, edge);
                }
            }
        }

        Graph {
            nodes,
            edges: edge_map.into_values().collect(),
        }
    }

    /// The number of clusters: connected components containing at least one
    /// edge (isolated nodes are not counted as clusters).
    pub fn cluster_count(&self) -> usize {
        let mut parent: Vec<usize> = (0..self.nodes.len()).collect();
        fn find(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        for edge in &self.edges {
            let a = find(&mut parent, edge.source);
            let b = find(&mut parent, edge.target);
            if a != b {
                parent[a] = b;
            }
        }
        let mut in_edge = vec![false; self.nodes.len()];
        for edge in &self.edges {
            in_edge[edge.source] = true;
            in_edge[edge.target] = true;
        }
        let mut roots: Vec<usize> = (0..self.nodes.len())
            .filter(|&i| in_edge[i])
            .map(|i| find(&mut parent, i))
            .collect();
        roots.sort_unstable();
        roots.dedup();
        roots.len()
    }
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

    const HEADERS: [&str; 9] = [
        "REFERENCE_ASSET_PATH",
        "CANDIDATE_ASSET_PATH",
        "MATCH_PERCENTAGE",
        "REF_XID",
        "CAN_XID",
        "REF_XUNITS",
        "CAN_XUNITS",
        "REF_XNAME",
        "CAN_XNAME",
    ];

    #[test]
    fn builds_nodes_and_edges_with_scores() {
        let g = Graph::from_report(&report(
            &HEADERS,
            vec![
                vec!["a.par", "b.par", "90", "u-a", "u-b", "mm", "mm", "A", "B"],
                vec!["b.par", "c.par", "85", "u-b", "", "mm", "in", "B", ""],
            ],
        ));
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 2);
        // XUNITS matches, XID/XNAME are unscored -> metadata 100%.
        assert_eq!(g.edges[0].metadata, Some(100.0));
        assert_eq!(g.edges[0].geometry, Some(90.0));
        // "mm" vs "in" -> 0% over one comparable field.
        assert_eq!(g.edges[1].metadata, Some(0.0));
        assert_eq!(g.edges[1].comparable, 1);
    }

    #[test]
    fn same_path_with_and_without_uuid_is_one_node() {
        // Row 1 gives b.par a UUID; row 2 mentions b.par with no metadata.
        let g = Graph::from_report(&report(
            &HEADERS,
            vec![
                vec!["a.par", "b.par", "90", "u-a", "u-b", "mm", "mm", "", ""],
                vec!["b.par", "c.par", "85", "", "", "", "", "", ""],
            ],
        ));
        assert_eq!(g.nodes.len(), 3);
        let b = g.nodes.iter().find(|n| n.path == "b.par").unwrap();
        assert_eq!(b.uuid.as_deref(), Some("u-b"));
    }

    #[test]
    fn reversed_duplicate_rows_merge_into_one_edge() {
        let g = Graph::from_report(&report(
            &HEADERS,
            vec![
                vec!["a.par", "b.par", "90", "u-a", "", "mm", "", "", ""],
                vec!["b.par", "a.par", "90", "u-b", "u-a", "mm", "mm", "", ""],
            ],
        ));
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
        // The reversed row has more comparable metadata, so it wins.
        assert_eq!(g.edges[0].comparable, 1);
        assert_eq!(g.edges[0].metadata, Some(100.0));
    }

    #[test]
    fn self_matches_are_skipped() {
        let g = Graph::from_report(&report(
            &HEADERS,
            vec![vec![
                "a.par", "a.par", "100", "u-a", "u-a", "mm", "mm", "", "",
            ]],
        ));
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn edge_without_shared_metadata_has_no_metadata_score() {
        let g = Graph::from_report(&report(
            &HEADERS,
            vec![vec!["a.par", "b.par", "88", "u-a", "", "mm", "", "A", ""]],
        ));
        assert_eq!(g.edges[0].metadata, None);
        assert_eq!(g.edges[0].comparable, 0);
        // The one-sided fields still appear in the comparison details.
        assert!(g.edges[0].fields.iter().any(|f| f.field == "XUNITS"));
    }

    #[test]
    fn node_label_prefers_name_field_over_path_basename() {
        let g = Graph::from_report(&report(
            &HEADERS,
            vec![vec![
                "dir/a.par",
                "dir/b.par",
                "90",
                "u-a",
                "u-b",
                "mm",
                "mm",
                "Nice Name",
                "",
            ]],
        ));
        let a = g.nodes.iter().find(|n| n.path == "dir/a.par").unwrap();
        let b = g.nodes.iter().find(|n| n.path == "dir/b.par").unwrap();
        assert_eq!(a.label(), "Nice Name");
        assert_eq!(b.label(), "b.par");
    }

    #[test]
    fn cluster_count_counts_connected_components() {
        let g = Graph::from_report(&report(
            &HEADERS,
            vec![
                vec!["a.par", "b.par", "90", "", "", "", "", "", ""],
                vec!["b.par", "c.par", "85", "", "", "", "", "", ""],
                vec!["x.par", "y.par", "95", "", "", "", "", "", ""],
            ],
        ));
        assert_eq!(g.nodes.len(), 5);
        assert_eq!(g.cluster_count(), 2);
    }
}
