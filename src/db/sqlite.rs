use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::types::*;
use super::{DatabaseDriver, DbError};
use crate::connection::profile::ConnectionProfile;

fn to_rusqlite_value(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Int(i) => rusqlite::types::Value::Integer(*i),
        Value::Float(f) => rusqlite::types::Value::Real(*f),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        Value::Bytes(b) => rusqlite::types::Value::Blob(b.clone()),
        Value::DateTime(dt) => rusqlite::types::Value::Text(dt.to_string()),
        // Safety: RawSql is created only by text_to_value() and is consumed
        // by the SQL builders (execute_insert/save_and_reload) before params are bound.
        Value::RawSql(_) => unreachable!("RawSql is consumed before reaching the driver"),
    }
}

pub struct SqliteDriver {
    conn: Arc<Mutex<Connection>>,
}

#[async_trait]
impl DatabaseDriver for SqliteDriver {
    async fn connect(profile: &ConnectionProfile, _password: &str) -> Result<Self, DbError> {
        let path = profile
            .file_path
            .as_deref()
            .ok_or_else(|| DbError::Connection("No file path specified for SQLite".into()))?;

        let conn = tokio::task::spawn_blocking({
            let path = path.to_string();
            move || Connection::open(&path).map_err(|e| DbError::Connection(e.to_string()))
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))??;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    async fn disconnect(&self) -> Result<(), DbError> {
        Ok(())
    }

    async fn databases(&self) -> Result<Vec<String>, DbError> {
        Ok(vec!["main".to_string()])
    }

    async fn tables(&self, _database: &str) -> Result<Vec<String>, DbError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .map_err(|e| DbError::Query(e.to_string()))?;
            let tables: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(tables)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn columns(&self, _database: &str, table: &str) -> Result<Vec<Column>, DbError> {
        let conn = self.conn.clone();
        let table = table.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let mut stmt = conn
                .prepare(&format!(
                    "PRAGMA table_info(\"{}\")",
                    table.replace('"', "\"\"")
                ))
                .map_err(|e| DbError::Query(e.to_string()))?;
            let columns: Vec<Column> = stmt
                .query_map([], |row| {
                    Ok(Column {
                        name: row.get(1)?,
                        data_type: row.get(2)?,
                        nullable: {
                            let notnull: i32 = row.get(3)?;
                            notnull == 0
                        },
                        default_value: row.get(4)?,
                        is_primary_key: {
                            let pk: i32 = row.get(5)?;
                            pk > 0
                        },
                        extra: String::new(),
                    })
                })
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(columns)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn indexes(&self, _database: &str, table: &str) -> Result<Vec<Index>, DbError> {
        let conn = self.conn.clone();
        let table = table.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let mut stmt = conn
                .prepare(&format!(
                    "PRAGMA index_list(\"{}\")",
                    table.replace('"', "\"\"")
                ))
                .map_err(|e| DbError::Query(e.to_string()))?;

            let index_info: Vec<(String, bool)> = stmt
                .query_map([], |row| {
                    let name: String = row.get(1)?;
                    // unique column is an integer in rusqlite (0 or 1)
                    let unique_int: i32 = row.get(2)?;
                    Ok((name, unique_int != 0))
                })
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            let mut indexes = vec![];
            for (name, unique) in index_info {
                let mut idx_stmt = conn
                    .prepare(&format!(
                        "PRAGMA index_info(\"{}\")",
                        name.replace('"', "\"\"")
                    ))
                    .map_err(|e| DbError::Query(e.to_string()))?;

                let columns: Vec<String> = idx_stmt
                    .query_map([], |row| row.get(2))
                    .map_err(|e| DbError::Query(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                indexes.push(Index {
                    name,
                    columns,
                    unique,
                });
            }
            Ok(indexes)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn query(&self, sql: &str, params: &[Value]) -> Result<QueryResult, DbError> {
        let conn = self.conn.clone();
        let sql = sql.to_string();
        let params: Vec<rusqlite::types::Value> = params.iter().map(to_rusqlite_value).collect();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let start = Instant::now();
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| DbError::Query(e.to_string()))?;

            let col_count = stmt.column_count();
            // column_name returns Result<&str> in rusqlite 0.31
            let columns: Vec<Column> = (0..col_count)
                .map(|i| Column {
                    name: stmt.column_name(i).unwrap_or("?").to_string(),
                    data_type: String::new(),
                    nullable: true,
                    default_value: None,
                    is_primary_key: false,
                    extra: String::new(),
                })
                .collect();

            let rows: Vec<Row> = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    let mut values = vec![];
                    for i in 0..col_count {
                        let val = match row.get_ref(i) {
                            Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                            Ok(rusqlite::types::ValueRef::Integer(n)) => Value::Int(n),
                            Ok(rusqlite::types::ValueRef::Real(f)) => Value::Float(f),
                            Ok(rusqlite::types::ValueRef::Text(t)) => {
                                Value::String(String::from_utf8_lossy(t).to_string())
                            }
                            Ok(rusqlite::types::ValueRef::Blob(b)) => Value::Bytes(b.to_vec()),
                            Err(_) => Value::Null,
                        };
                        values.push(val);
                    }
                    Ok(values)
                })
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            let execution_time_ms = start.elapsed().as_millis() as u64;

            Ok(QueryResult {
                columns,
                rows,
                affected_rows: 0,
                execution_time_ms,
            })
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, DbError> {
        let conn = self.conn.clone();
        let sql = sql.to_string();
        let params: Vec<rusqlite::types::Value> = params.iter().map(to_rusqlite_value).collect();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let affected = conn
                .execute(&sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| DbError::Query(e.to_string()))?;
            Ok(affected as u64)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_create_and_query() {
        let tmp = std::env::temp_dir().join("querybox_test.db");
        let path = tmp.to_str().unwrap();
        let _ = std::fs::remove_file(path);

        let conn = Connection::open(path).unwrap();
        let driver = SqliteDriver {
            conn: Arc::new(Mutex::new(conn)),
        };

        driver
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)", &[])
            .await
            .unwrap();
        driver
            .execute("INSERT INTO test (name) VALUES ('alice')", &[])
            .await
            .unwrap();

        let result = driver.query("SELECT * FROM test", &[]).await.unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.columns[0].name, "id");
        assert_eq!(result.columns[1].name, "name");

        let tables = driver.tables("main").await.unwrap();
        assert!(tables.contains(&"test".to_string()));

        let _ = std::fs::remove_file(path);
    }
}
