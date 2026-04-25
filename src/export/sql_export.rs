use crate::db::types::{QueryResult, Value};

pub fn export(result: &QueryResult, table_name: &str) -> String {
    let mut lines = vec![];
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    let cols_joined = col_names.join(", ");

    for row in &result.rows {
        let values: Vec<String> = row.iter().map(sql_literal).collect();
        lines.push(format!(
            "INSERT INTO {} ({}) VALUES ({});",
            table_name,
            cols_joined,
            values.join(", ")
        ));
    }

    lines.join("\n")
}

fn sql_literal(v: &Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Bytes(b) => format!("X'{}'", hex::encode(b)),
        Value::DateTime(dt) => format!("'{}'", dt.format("%Y-%m-%d %H:%M:%S")),
        Value::RawSql(expr) => expr.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::{Column, Value};

    #[test]
    fn test_sql_export() {
        let result = QueryResult {
            columns: vec![
                Column {
                    name: "id".into(),
                    data_type: "INT".into(),
                    nullable: false,
                    default_value: None,
                    is_primary_key: true,
                    extra: String::new(),
                },
                Column {
                    name: "name".into(),
                    data_type: "VARCHAR".into(),
                    nullable: false,
                    default_value: None,
                    is_primary_key: false,
                    extra: String::new(),
                },
            ],
            rows: vec![vec![Value::Int(1), Value::String("alice".into())]],
            affected_rows: 0,
            execution_time_ms: 5,
        };
        let output = export(&result, "users");
        assert_eq!(output, "INSERT INTO users (id, name) VALUES (1, 'alice');");
    }

    #[test]
    fn test_sql_escaping() {
        let val = Value::String("O'Brien".into());
        assert_eq!(sql_literal(&val), "'O''Brien'");
    }
}
