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
    RawSql(String),
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
            Value::RawSql(expr) => write!(f, "{}", expr),
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

/// Returns true if `s` looks like a SQL expression rather than a literal value.
///
/// Uses a keyword list and a function-call heuristic. Known trade-off: strings
/// like `"John (Jr)"` (starts with alpha, contains `(`, ends with `)`) will be
/// detected as SQL expressions. This is accepted behaviour — users who need to
/// store such strings can use the raw SQL editor.
#[allow(dead_code)]
pub fn is_sql_expression(s: &str) -> bool {
    let t = s.trim();
    const KEYWORDS: &[&str] = &[
        "NULL",
        "DEFAULT",
        "TRUE",
        "FALSE",
        "CURRENT_TIMESTAMP",
        "CURRENT_DATE",
        "CURRENT_TIME",
    ];
    if KEYWORDS.iter().any(|k| t.eq_ignore_ascii_case(k)) {
        return true;
    }
    let first = t.chars().next();
    matches!(first, Some(c) if c.is_ascii_alphabetic() || c == '_')
        && t.contains('(')
        && t.ends_with(')')
}

/// Convert user-typed text into the appropriate `Value`.
#[allow(dead_code)]
pub fn text_to_value(s: &str) -> Value {
    if is_sql_expression(s) {
        Value::RawSql(s.trim().to_string())
    } else {
        Value::String(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sql_expression_keywords() {
        assert!(is_sql_expression("NULL"));
        assert!(is_sql_expression("null"));
        assert!(is_sql_expression("  NULL  "));
        assert!(is_sql_expression("DEFAULT"));
        assert!(is_sql_expression("CURRENT_TIMESTAMP"));
        assert!(is_sql_expression("current_date"));
        assert!(is_sql_expression("TRUE"));
        assert!(is_sql_expression("FALSE"));
    }

    #[test]
    fn test_is_sql_expression_functions() {
        assert!(is_sql_expression("NOW()"));
        assert!(is_sql_expression("now()"));
        assert!(is_sql_expression("UUID()"));
        assert!(is_sql_expression("DATE_ADD(NOW(), INTERVAL 1 DAY)"));
        assert!(is_sql_expression("COALESCE(NULL, 0)"));
    }

    #[test]
    fn test_is_sql_expression_plain_strings() {
        assert!(!is_sql_expression("hello"));
        assert!(!is_sql_expression("123"));
        assert!(!is_sql_expression("some text with spaces"));
        assert!(!is_sql_expression(""));
        assert!(!is_sql_expression("O'Brien"));
    }

    #[test]
    fn test_text_to_value_sql() {
        assert_eq!(text_to_value("NOW()"), Value::RawSql("NOW()".to_string()));
        assert_eq!(text_to_value("NULL"), Value::RawSql("NULL".to_string()));
        assert_eq!(text_to_value("  NOW()  "), Value::RawSql("NOW()".to_string()));
    }

    #[test]
    fn test_text_to_value_string() {
        assert_eq!(text_to_value("hello"), Value::String("hello".to_string()));
        assert_eq!(text_to_value(""), Value::String("".to_string()));
    }
}
