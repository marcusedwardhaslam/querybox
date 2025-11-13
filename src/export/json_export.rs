use super::ExportError;
use crate::db::types::QueryResult;
use serde_json::{json, Map};

pub fn export(result: &QueryResult) -> Result<String, ExportError> {
    let rows: Vec<serde_json::Value> = result
        .rows
        .iter()
        .map(|row| {
            let mut obj = Map::new();
            for (i, val) in row.iter().enumerate() {
                let col_name = result
                    .columns
                    .get(i)
                    .map(|c| c.name.as_str())
                    .unwrap_or("unknown");
                obj.insert(col_name.to_string(), value_to_json(val));
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    serde_json::to_string_pretty(&rows).map_err(ExportError::Json)
}

fn value_to_json(v: &crate::db::types::Value) -> serde_json::Value {
    use crate::db::types::Value;
    match v {
        Value::Null => json!(null),
        Value::Bool(b) => json!(b),
        Value::Int(i) => json!(i),
        Value::Float(f) => json!(f),
        Value::String(s) => json!(s),
        Value::Bytes(b) => json!(format!("<{} bytes>", b.len())),
        Value::DateTime(dt) => json!(dt.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::{Column, Value};

    #[test]
    fn test_json_export() {
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
        let output = export(&result).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["id"], 1);
        assert_eq!(parsed[0]["name"], "alice");
    }
}
