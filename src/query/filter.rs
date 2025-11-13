use crate::db::types::{Dialect, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Like,
    NotLike,
    StartsWith,
    EndsWith,
    Contains,
    NotContains,
    IsNull,
    IsNotNull,
    In,
    NotIn,
    Between,
}

impl FilterOp {
    pub fn all() -> &'static [FilterOp] {
        &[
            FilterOp::Equals,
            FilterOp::NotEquals,
            FilterOp::GreaterThan,
            FilterOp::LessThan,
            FilterOp::GreaterOrEqual,
            FilterOp::LessOrEqual,
            FilterOp::Like,
            FilterOp::NotLike,
            FilterOp::StartsWith,
            FilterOp::EndsWith,
            FilterOp::Contains,
            FilterOp::NotContains,
            FilterOp::IsNull,
            FilterOp::IsNotNull,
            FilterOp::In,
            FilterOp::NotIn,
            FilterOp::Between,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            FilterOp::Equals => "= equals",
            FilterOp::NotEquals => "≠ not equals",
            FilterOp::GreaterThan => "> greater than",
            FilterOp::LessThan => "< less than",
            FilterOp::GreaterOrEqual => "≥ greater or equal",
            FilterOp::LessOrEqual => "≤ less or equal",
            FilterOp::Like => "~ LIKE pattern",
            FilterOp::NotLike => "!~ NOT LIKE pattern",
            FilterOp::StartsWith => "starts with",
            FilterOp::EndsWith => "ends with",
            FilterOp::Contains => "contains",
            FilterOp::NotContains => "does not contain",
            FilterOp::IsNull => "is null",
            FilterOp::IsNotNull => "is not null",
            FilterOp::In => "IN (a, b, ...)",
            FilterOp::NotIn => "NOT IN (a, b, ...)",
            FilterOp::Between => "BETWEEN a AND b",
        }
    }

    pub fn needs_value(&self) -> bool {
        !matches!(self, FilterOp::IsNull | FilterOp::IsNotNull)
    }
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    pub value: Option<String>,
}

impl Filter {
    pub fn summary(&self) -> String {
        match &self.op {
            FilterOp::IsNull => format!("{} is null", self.column),
            FilterOp::IsNotNull => format!("{} is not null", self.column),
            op => format!(
                "{} {} {}",
                self.column,
                op.label(),
                self.value.as_deref().unwrap_or("")
            ),
        }
    }
}

/// Build a WHERE clause and parameter list from filters.
/// Returns (where_clause_with_WHERE_keyword, params).
/// Uses `?` placeholders (MySQL-compatible).
pub fn filters_to_sql(filters: &[Filter], dialect: Dialect) -> (String, Vec<Value>) {
    if filters.is_empty() {
        return (String::new(), vec![]);
    }

    let mut conditions = vec![];
    let mut params = vec![];

    for filter in filters {
        let col = dialect.quote_identifier(&filter.column);
        let raw = filter.value.clone().unwrap_or_default();

        match &filter.op {
            FilterOp::IsNull => {
                conditions.push(format!("{col} IS NULL"));
            }
            FilterOp::IsNotNull => {
                conditions.push(format!("{col} IS NOT NULL"));
            }
            FilterOp::Equals => {
                conditions.push(format!("{col} = ?"));
                params.push(Value::String(raw));
            }
            FilterOp::NotEquals => {
                conditions.push(format!("{col} != ?"));
                params.push(Value::String(raw));
            }
            FilterOp::GreaterThan => {
                conditions.push(format!("{col} > ?"));
                params.push(Value::String(raw));
            }
            FilterOp::LessThan => {
                conditions.push(format!("{col} < ?"));
                params.push(Value::String(raw));
            }
            FilterOp::GreaterOrEqual => {
                conditions.push(format!("{col} >= ?"));
                params.push(Value::String(raw));
            }
            FilterOp::LessOrEqual => {
                conditions.push(format!("{col} <= ?"));
                params.push(Value::String(raw));
            }
            FilterOp::Like => {
                conditions.push(format!("{col} LIKE ?"));
                params.push(Value::String(raw));
            }
            FilterOp::NotLike => {
                conditions.push(format!("{col} NOT LIKE ?"));
                params.push(Value::String(raw));
            }
            FilterOp::StartsWith => {
                conditions.push(format!("{col} LIKE ?"));
                params.push(Value::String(format!("{raw}%")));
            }
            FilterOp::EndsWith => {
                conditions.push(format!("{col} LIKE ?"));
                params.push(Value::String(format!("%{raw}")));
            }
            FilterOp::Contains => {
                conditions.push(format!("{col} LIKE ?"));
                params.push(Value::String(format!("%{raw}%")));
            }
            FilterOp::NotContains => {
                conditions.push(format!("{col} NOT LIKE ?"));
                params.push(Value::String(format!("%{raw}%")));
            }
            FilterOp::In => {
                let parts: Vec<&str> = raw.split(',').map(|s| s.trim()).collect();
                let placeholders = parts.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                conditions.push(format!("{col} IN ({placeholders})"));
                for p in parts {
                    params.push(Value::String(p.to_string()));
                }
            }
            FilterOp::NotIn => {
                let parts: Vec<&str> = raw.split(',').map(|s| s.trim()).collect();
                let placeholders = parts.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                conditions.push(format!("{col} NOT IN ({placeholders})"));
                for p in parts {
                    params.push(Value::String(p.to_string()));
                }
            }
            FilterOp::Between => {
                let mut parts = raw.splitn(2, ',').map(|s| s.trim().to_string());
                let a = parts.next().unwrap_or_default();
                let b = parts.next().unwrap_or_default();
                conditions.push(format!("{col} BETWEEN ? AND ?"));
                params.push(Value::String(a));
                params.push(Value::String(b));
            }
        }
    }

    let clause = format!("WHERE {}", conditions.join(" AND "));
    (clause, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filters() {
        let (clause, params) = filters_to_sql(&[], Dialect::MySql);
        assert_eq!(clause, "");
        assert!(params.is_empty());
    }

    #[test]
    fn test_equals_filter_mysql() {
        let filters = vec![Filter {
            column: "username".to_string(),
            op: FilterOp::Equals,
            value: Some("alice".to_string()),
        }];
        let (clause, params) = filters_to_sql(&filters, Dialect::MySql);
        assert_eq!(clause, "WHERE `username` = ?");
        assert_eq!(params, vec![Value::String("alice".to_string())]);
    }

    #[test]
    fn test_in_filter() {
        let filters = vec![Filter {
            column: "status".to_string(),
            op: FilterOp::In,
            value: Some("active, inactive".to_string()),
        }];
        let (clause, params) = filters_to_sql(&filters, Dialect::MySql);
        assert_eq!(clause, "WHERE `status` IN (?, ?)");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_between_filter() {
        let filters = vec![Filter {
            column: "age".to_string(),
            op: FilterOp::Between,
            value: Some("18, 65".to_string()),
        }];
        let (clause, params) = filters_to_sql(&filters, Dialect::MySql);
        assert_eq!(clause, "WHERE `age` BETWEEN ? AND ?");
        assert_eq!(params.len(), 2);
    }
}
