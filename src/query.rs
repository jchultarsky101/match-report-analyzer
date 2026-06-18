//! The structured filter builder.
//!
//! This models the point-and-click query UI: a list of [`Condition`]s joined by
//! a single [`Combinator`] (AND/OR). It compiles to a SQL `WHERE` clause that
//! runs against the [`crate::store`] table, so the builder and the raw SQL box
//! share one execution path.

use std::fmt;

use crate::store::{Column, ColumnType, quote_ident};

/// A comparison operator offered by the filter builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
    Contains,
    StartsWith,
    IsEmpty,
    IsNotEmpty,
}

impl Operator {
    /// Every operator, in display order — used to populate the operator dropdown.
    pub const ALL: [Operator; 10] = [
        Operator::Equals,
        Operator::NotEquals,
        Operator::GreaterThan,
        Operator::GreaterOrEqual,
        Operator::LessThan,
        Operator::LessOrEqual,
        Operator::Contains,
        Operator::StartsWith,
        Operator::IsEmpty,
        Operator::IsNotEmpty,
    ];

    /// Whether this operator compares against a user-supplied value.
    pub fn needs_value(self) -> bool {
        !matches!(self, Operator::IsEmpty | Operator::IsNotEmpty)
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Operator::Equals => "=",
            Operator::NotEquals => "≠",
            Operator::GreaterThan => ">",
            Operator::GreaterOrEqual => "≥",
            Operator::LessThan => "<",
            Operator::LessOrEqual => "≤",
            Operator::Contains => "contains",
            Operator::StartsWith => "starts with",
            Operator::IsEmpty => "is empty",
            Operator::IsNotEmpty => "is not empty",
        };
        f.write_str(s)
    }
}

/// How multiple conditions are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Combinator {
    #[default]
    And,
    Or,
}

impl Combinator {
    pub const ALL: [Combinator; 2] = [Combinator::And, Combinator::Or];

    fn sql(self) -> &'static str {
        match self {
            Combinator::And => " AND ",
            Combinator::Or => " OR ",
        }
    }
}

impl fmt::Display for Combinator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Combinator::And => "AND",
            Combinator::Or => "OR",
        })
    }
}

/// A single filter row: column, operator, and (for most operators) a value.
#[derive(Debug, Clone, Default)]
pub struct Condition {
    pub column: Option<String>,
    pub operator: Option<Operator>,
    pub value: String,
}

impl Condition {
    /// Compile this condition to a SQL boolean expression, or `None` if it is
    /// incomplete (no column/operator chosen, or a value-requiring operator
    /// with an empty value).
    fn to_sql(&self, columns: &[Column]) -> Option<String> {
        let column = self.column.as_ref()?;
        let operator = self.operator?;
        let ident = quote_ident(column);
        let ty = columns
            .iter()
            .find(|c| &c.name == column)
            .map(|c| c.ty)
            .unwrap_or(ColumnType::Text);

        match operator {
            Operator::IsEmpty => Some(format!("({ident} IS NULL OR {ident} = '')")),
            Operator::IsNotEmpty => Some(format!("({ident} IS NOT NULL AND {ident} <> '')")),
            Operator::Contains => {
                let v = self.non_empty_value()?;
                Some(format!("{ident} LIKE '%{}%'", escape_like(v)))
            }
            Operator::StartsWith => {
                let v = self.non_empty_value()?;
                Some(format!("{ident} LIKE '{}%'", escape_like(v)))
            }
            _ => {
                let v = self.non_empty_value()?;
                let sql_op = match operator {
                    Operator::Equals => "=",
                    Operator::NotEquals => "<>",
                    Operator::GreaterThan => ">",
                    Operator::GreaterOrEqual => ">=",
                    Operator::LessThan => "<",
                    Operator::LessOrEqual => "<=",
                    _ => unreachable!("handled above"),
                };
                Some(format!("{ident} {sql_op} {}", format_literal(v, ty)))
            }
        }
    }

    fn non_empty_value(&self) -> Option<&str> {
        let v = self.value.trim();
        (!v.is_empty()).then_some(v)
    }
}

/// The full structured query: conditions joined by one combinator.
#[derive(Debug, Clone, Default)]
pub struct FilterBuilder {
    pub conditions: Vec<Condition>,
    pub combinator: Combinator,
}

impl FilterBuilder {
    /// Build the `WHERE` clause body (without the `WHERE` keyword) from all
    /// complete conditions, or `None` if there are none.
    pub fn where_clause(&self, columns: &[Column]) -> Option<String> {
        let parts: Vec<String> = self
            .conditions
            .iter()
            .filter_map(|c| c.to_sql(columns))
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(self.combinator.sql()))
        }
    }
}

/// Format a comparison literal: numbers bare for numeric columns, otherwise a
/// quoted, escaped string literal.
fn format_literal(value: &str, ty: ColumnType) -> String {
    if ty.is_numeric() && value.parse::<f64>().is_ok() {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

/// Escape a value embedded inside a `LIKE` string literal.
fn escape_like(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns() -> Vec<Column> {
        vec![
            Column {
                name: "MATCH_PERCENTAGE".into(),
                ty: ColumnType::Real,
            },
            Column {
                name: "Material".into(),
                ty: ColumnType::Text,
            },
        ]
    }

    #[test]
    fn builds_the_steel_example_clause() {
        let builder = FilterBuilder {
            combinator: Combinator::And,
            conditions: vec![
                Condition {
                    column: Some("MATCH_PERCENTAGE".into()),
                    operator: Some(Operator::GreaterThan),
                    value: "80".into(),
                },
                Condition {
                    column: Some("MATCH_PERCENTAGE".into()),
                    operator: Some(Operator::LessThan),
                    value: "99".into(),
                },
                Condition {
                    column: Some("Material".into()),
                    operator: Some(Operator::Equals),
                    value: "Steel".into(),
                },
            ],
        };
        let clause = builder.where_clause(&columns()).unwrap();
        assert_eq!(
            clause,
            "\"MATCH_PERCENTAGE\" > 80 AND \"MATCH_PERCENTAGE\" < 99 AND \"Material\" = 'Steel'"
        );
    }

    #[test]
    fn skips_incomplete_conditions() {
        let builder = FilterBuilder {
            combinator: Combinator::And,
            conditions: vec![Condition::default()],
        };
        assert!(builder.where_clause(&columns()).is_none());
    }

    #[test]
    fn escapes_string_literals() {
        let builder = FilterBuilder {
            combinator: Combinator::And,
            conditions: vec![Condition {
                column: Some("Material".into()),
                operator: Some(Operator::Equals),
                value: "O'Brien".into(),
            }],
        };
        let clause = builder.where_clause(&columns()).unwrap();
        assert_eq!(clause, "\"Material\" = 'O''Brien'");
    }
}
