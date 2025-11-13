use super::ExportError;
use crate::db::types::QueryResult;

pub fn export(result: &QueryResult) -> Result<String, ExportError> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    let headers: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    wtr.write_record(&headers)
        .map_err(|e| ExportError::Csv(e.to_string()))?;

    for row in &result.rows {
        let fields: Vec<String> = row.iter().map(|v| v.to_string()).collect();
        wtr.write_record(&fields)
            .map_err(|e| ExportError::Csv(e.to_string()))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| ExportError::Csv(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| ExportError::Csv(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::{Column, Value};

    fn sample_result() -> QueryResult {
        QueryResult {
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
            rows: vec![
                vec![Value::Int(1), Value::String("alice".into())],
                vec![Value::Int(2), Value::String("bob".into())],
            ],
            affected_rows: 0,
            execution_time_ms: 5,
        }
    }

    #[test]
    fn test_csv_export() {
        let result = export(&sample_result()).unwrap();
        assert!(result.contains("id,name"));
        assert!(result.contains("1,alice"));
        assert!(result.contains("2,bob"));
    }
}
