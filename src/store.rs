//! In-memory data store backing the application.
//!
//! A loaded match report is held in an in-memory SQLite database as a single
//! table named [`TABLE`]. The CSV is read once, column types are inferred, and
//! all querying (both the structured filter builder and the raw SQL box) runs
//! against this table. The source file is only ever read, never modified.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use rusqlite::types::{Value, ValueRef};

/// Name of the table every loaded report is stored under.
pub const TABLE: &str = "report";

/// Inferred storage type of a column, used to drive both the SQLite schema and
/// the operators offered by the filter builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Real,
    Text,
}

impl ColumnType {
    /// SQLite column declaration keyword.
    fn sql_decl(self) -> &'static str {
        match self {
            ColumnType::Integer => "INTEGER",
            ColumnType::Real => "REAL",
            ColumnType::Text => "TEXT",
        }
    }

    /// Whether this column supports numeric comparison operators.
    pub fn is_numeric(self) -> bool {
        matches!(self, ColumnType::Integer | ColumnType::Real)
    }
}

/// A single column's name and inferred type.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub ty: ColumnType,
}

/// The result of running a query: column headers plus rows of display strings.
#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A loaded report held in an in-memory SQLite database.
pub struct DataStore {
    conn: Connection,
    columns: Vec<Column>,
}

impl DataStore {
    /// Load a CSV match report from `path` into a fresh in-memory database.
    pub fn load_csv(path: &Path) -> Result<Self> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(path)
            .with_context(|| format!("opening CSV file {}", path.display()))?;

        let headers: Vec<String> = reader
            .headers()
            .context("reading CSV header row")?
            .iter()
            .map(|h| h.to_string())
            .collect();

        anyhow::ensure!(!headers.is_empty(), "CSV file has no columns");

        let records: Vec<csv::StringRecord> = reader
            .records()
            .collect::<std::result::Result<_, _>>()
            .context("reading CSV rows")?;

        let types = infer_types(&headers, &records);
        let columns: Vec<Column> = headers
            .iter()
            .zip(&types)
            .map(|(name, &ty)| Column {
                name: name.clone(),
                ty,
            })
            .collect();

        let conn = Connection::open_in_memory().context("creating in-memory database")?;
        create_table(&conn, &columns)?;
        insert_rows(&conn, &columns, &records)?;

        Ok(Self { conn, columns })
    }

    /// The columns of the loaded report, in their original order.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Number of rows in the loaded report.
    pub fn row_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row(&format!("SELECT COUNT(*) FROM {TABLE}"), [], |row| {
                row.get(0)
            })
            .context("counting rows")?;
        Ok(count as usize)
    }

    /// Run an arbitrary read-only SQL query and collect the result as strings.
    pub fn query(&self, sql: &str) -> Result<QueryResult> {
        let mut stmt = self.conn.prepare(sql).context("preparing query")?;
        let column_count = stmt.column_count();
        let columns: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect();

        let mut rows = Vec::new();
        let mut query_rows = stmt.query([]).context("executing query")?;
        while let Some(row) = query_rows.next().context("reading result row")? {
            let mut cells = Vec::with_capacity(column_count);
            for i in 0..column_count {
                cells.push(value_to_string(row.get_ref(i)?));
            }
            rows.push(cells);
        }

        Ok(QueryResult { columns, rows })
    }
}

/// Infer a [`ColumnType`] per column from the data: a column is `Integer` if
/// every non-empty value parses as `i64`, `Real` if every non-empty value
/// parses as `f64`, otherwise `Text`. An all-empty column defaults to `Text`.
fn infer_types(headers: &[String], records: &[csv::StringRecord]) -> Vec<ColumnType> {
    let mut all_integer = vec![true; headers.len()];
    let mut all_real = vec![true; headers.len()];
    let mut any_value = vec![false; headers.len()];

    for record in records {
        for (i, field) in record.iter().enumerate() {
            if i >= headers.len() {
                break;
            }
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            any_value[i] = true;
            if field.parse::<i64>().is_err() {
                all_integer[i] = false;
            }
            if field.parse::<f64>().is_err() {
                all_real[i] = false;
            }
        }
    }

    (0..headers.len())
        .map(|i| {
            if !any_value[i] {
                ColumnType::Text
            } else if all_integer[i] {
                ColumnType::Integer
            } else if all_real[i] {
                ColumnType::Real
            } else {
                ColumnType::Text
            }
        })
        .collect()
}

/// Create the report table with one quoted column per header.
fn create_table(conn: &Connection, columns: &[Column]) -> Result<()> {
    let cols = columns
        .iter()
        .map(|c| format!("{} {}", quote_ident(&c.name), c.ty.sql_decl()))
        .collect::<Vec<_>>()
        .join(", ");
    conn.execute_batch(&format!("CREATE TABLE {TABLE} ({cols});"))
        .context("creating report table")?;
    Ok(())
}

/// Bulk-insert every record into the report table inside a single transaction.
fn insert_rows(conn: &Connection, columns: &[Column], records: &[csv::StringRecord]) -> Result<()> {
    let placeholders = vec!["?"; columns.len()].join(", ");
    let sql = format!("INSERT INTO {TABLE} VALUES ({placeholders})");

    conn.execute_batch("BEGIN")?;
    {
        let mut stmt = conn.prepare(&sql).context("preparing insert")?;
        for record in records {
            let values: Vec<Value> = columns
                .iter()
                .enumerate()
                .map(|(i, col)| cell_to_value(record.get(i).unwrap_or("").trim(), col.ty))
                .collect();
            stmt.execute(rusqlite::params_from_iter(values))
                .context("inserting row")?;
        }
    }
    conn.execute_batch("COMMIT")?;
    Ok(())
}

/// Convert a raw CSV cell into a typed SQLite [`Value`]; empty cells become NULL.
fn cell_to_value(field: &str, ty: ColumnType) -> Value {
    if field.is_empty() {
        return Value::Null;
    }
    match ty {
        ColumnType::Integer => field
            .parse::<i64>()
            .map(Value::Integer)
            .unwrap_or_else(|_| Value::Text(field.to_string())),
        ColumnType::Real => field
            .parse::<f64>()
            .map(Value::Real)
            .unwrap_or_else(|_| Value::Text(field.to_string())),
        ColumnType::Text => Value::Text(field.to_string()),
    }
}

/// Render a SQLite value as a display string for the grid (NULL becomes empty).
fn value_to_string(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(r) => r.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(_) => "<blob>".to_string(),
    }
}

/// Quote a SQL identifier by wrapping it in double quotes and escaping any
/// embedded double quotes — needed because report columns contain characters
/// like spaces and `($)`.
pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_identifiers_with_special_characters() {
        assert_eq!(quote_ident("MATCH_PERCENTAGE"), "\"MATCH_PERCENTAGE\"");
        assert_eq!(quote_ident("REF__COST_($)"), "\"REF__COST_($)\"");
        assert_eq!(quote_ident(r#"a"b"#), "\"a\"\"b\"");
    }

    #[test]
    fn infers_column_types() {
        let headers = vec!["score".to_string(), "id".to_string(), "label".to_string()];
        let mut r1 = csv::StringRecord::new();
        r1.push_field("89.7");
        r1.push_field("214");
        r1.push_field("steel");
        let mut r2 = csv::StringRecord::new();
        r2.push_field("94.2");
        r2.push_field("215");
        r2.push_field("");
        let types = infer_types(&headers, &[r1, r2]);
        assert_eq!(
            types,
            vec![ColumnType::Real, ColumnType::Integer, ColumnType::Text]
        );
    }
}
