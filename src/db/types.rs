use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A single value from a database cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    DateTime(NaiveDateTime),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(b) => write!(f, "<{} bytes>", b.len()),
            Value::DateTime(dt) => write!(f, "{}", dt),
        }
    }
}

/// Metadata about a single column in a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub extra: String,
}

/// An index on a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

/// A foreign key constraint on a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ForeignKey {
    pub column: String,
    pub ref_database: String,
    pub ref_table: String,
    pub ref_column: String,
}

/// A single row of query results.
pub type Row = Vec<Value>;

/// The result of a query execution.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    #[allow(dead_code)]
    pub affected_rows: u64,
    pub execution_time_ms: u64,
}

impl QueryResult {
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            columns: vec![],
            rows: vec![],
            affected_rows: 0,
            execution_time_ms: 0,
        }
    }
}

/// SQL dialect for quoting identifiers and generating SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    MySql,
    PostgreSql,
    Sqlite,
}

impl Dialect {
    pub fn quote_identifier(&self, name: &str) -> String {
        match self {
            Dialect::MySql => format!("`{}`", name.replace('`', "``")),
            Dialect::PostgreSql => format!("\"{}\"", name.replace('"', "\"\"")),
            Dialect::Sqlite => format!("\"{}\"", name.replace('"', "\"\"")),
        }
    }
}
