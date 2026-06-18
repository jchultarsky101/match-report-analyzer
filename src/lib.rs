//! match-report-analyzer — library core.
//!
//! The GUI binary (`src/main.rs`) is a thin Iced shell over these modules:
//!
//! - [`store`] loads a Physna match-report CSV into an in-memory SQLite table
//!   and runs read-only queries against it.
//! - [`query`] models the structured filter builder and compiles it to SQL.
//!
//! Splitting the logic into a library keeps it unit- and integration-testable
//! independently of the UI.

pub mod query;
pub mod store;
