use async_trait::async_trait;
use mysql_async::consts::ColumnFlags;
use mysql_async::prelude::*;
use mysql_async::{Conn, Opts, OptsBuilder, Pool};
use std::time::Instant;

use super::types::*;
use super::{DatabaseDriver, DbError};
use crate::connection::profile::ConnectionProfile;

pub struct MySqlDriver {
    pool: Option<Pool>,
}

impl Drop for MySqlDriver {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.take() {
            // Pool destructor needs a tokio context. Ensure one is always available.
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    handle.spawn(async move {
                        pool.disconnect().await.ok();
                    });
                }
                Err(_) => {
                    crate::db_runtime().block_on(async move {
                        pool.disconnect().await.ok();
                    });
                }
            }
        }
    }
}

impl MySqlDriver {
    fn opts_from_profile(profile: &ConnectionProfile, password: &str) -> Opts {
        OptsBuilder::default()
            .ip_or_hostname(profile.host.clone())
            .tcp_port(profile.port)
            .user(Some(profile.user.clone()))
            .pass(Some(password.to_string()))
            .db_name(profile.default_database.clone())
            .into()
    }

    fn pool(&self) -> Result<&Pool, DbError> {
        self.pool.as_ref().ok_or(DbError::NotConnected)
    }

    async fn get_conn(&self) -> Result<Conn, DbError> {
        self.pool()?
            .get_conn()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))
    }
}

#[async_trait]
impl DatabaseDriver for MySqlDriver {
    async fn connect(profile: &ConnectionProfile, password: &str) -> Result<Self, DbError> {
        let opts = Self::opts_from_profile(profile, password);
        let pool = Pool::new(opts);
        pool.get_conn()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;
        Ok(Self { pool: Some(pool) })
    }

    async fn disconnect(&self) -> Result<(), DbError> {
        if let Some(pool) = &self.pool {
            pool.clone()
                .disconnect()
                .await
                .map_err(|e| DbError::Other(e.to_string()))?;
        }
        Ok(())
    }

    async fn databases(&self) -> Result<Vec<String>, DbError> {
        let mut conn = self.get_conn().await?;
        let rows: Vec<String> = conn
            .query("SHOW DATABASES")
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows)
    }

    async fn tables(&self, database: &str) -> Result<Vec<String>, DbError> {
        let mut conn = self.get_conn().await?;
        let query = "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES \
                     WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME";
        let rows: Vec<String> = conn
            .exec(query, (database,))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows)
    }

    async fn columns(&self, database: &str, table: &str) -> Result<Vec<Column>, DbError> {
        let mut conn = self.get_conn().await?;
        let query =
            "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_DEFAULT, COLUMN_KEY, EXTRA \
                     FROM INFORMATION_SCHEMA.COLUMNS \
                     WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                     ORDER BY ORDINAL_POSITION";
        let rows: Vec<(String, String, String, Option<String>, String, String)> = conn
            .exec(query, (database, table))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(
                |(name, data_type, nullable, default_value, key, extra)| Column {
                    name,
                    data_type,
                    nullable: nullable == "YES",
                    default_value,
                    is_primary_key: key == "PRI",
                    extra,
                },
            )
            .collect())
    }

    async fn indexes(&self, database: &str, table: &str) -> Result<Vec<Index>, DbError> {
        let mut conn = self.get_conn().await?;
        let query = "SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE \
                     FROM INFORMATION_SCHEMA.STATISTICS \
                     WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                     ORDER BY INDEX_NAME, SEQ_IN_INDEX";
        let rows: Vec<(String, String, i32)> = conn
            .exec(query, (database, table))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let mut indexes: Vec<Index> = vec![];
        for (name, column, non_unique) in rows {
            if let Some(idx) = indexes.iter_mut().find(|i| i.name == name) {
                idx.columns.push(column);
            } else {
                indexes.push(Index {
                    name,
                    columns: vec![column],
                    unique: non_unique == 0,
                });
            }
        }
        Ok(indexes)
    }

    async fn foreign_keys(&self, database: &str, table: &str) -> Result<Vec<ForeignKey>, DbError> {
        let mut conn = self.get_conn().await?;
        let query = "SELECT COLUMN_NAME, REFERENCED_TABLE_SCHEMA, \
                             REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
                     FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE \
                     WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                       AND REFERENCED_TABLE_NAME IS NOT NULL \
                     ORDER BY COLUMN_NAME";
        let rows: Vec<(String, String, String, String)> = conn
            .exec(query, (database, table))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|(column, ref_database, ref_table, ref_column)| ForeignKey {
                column,
                ref_database,
                ref_table,
                ref_column,
            })
            .collect())
    }

    async fn query(&self, sql: &str, params: &[Value]) -> Result<QueryResult, DbError> {
        let mut conn = self.get_conn().await?;
        exec_query(&mut conn, sql, params).await
    }

    async fn query_in(
        &self,
        database: Option<&str>,
        sql: &str,
        params: &[Value],
    ) -> Result<QueryResult, DbError> {
        let mut conn = self.get_conn().await?;
        if let Some(db) = database {
            conn.query_drop(format!("USE `{}`", db))
                .await
                .map_err(|e| DbError::Query(e.to_string()))?;
        }
        exec_query(&mut conn, sql, params).await
    }

    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, DbError> {
        let mut conn = self.get_conn().await?;
        let mysql_params = values_to_mysql_params(params);
        conn.exec_drop(sql, mysql_params)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(conn.affected_rows())
    }

    fn dialect(&self) -> Dialect {
        Dialect::MySql
    }
}

async fn exec_query(conn: &mut Conn, sql: &str, params: &[Value]) -> Result<QueryResult, DbError> {
    let start = Instant::now();
    let mysql_params = values_to_mysql_params(params);
    let result: Vec<mysql_async::Row> = conn
        .exec(sql, mysql_params)
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;
    let execution_time_ms = start.elapsed().as_millis() as u64;

    if result.is_empty() {
        return Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: 0,
            execution_time_ms,
        });
    }

    let columns: Vec<Column> = result[0]
        .columns_ref()
        .iter()
        .map(|c| Column {
            name: c.name_str().to_string(),
            data_type: format!("{:?}", c.column_type()),
            nullable: !c.flags().contains(ColumnFlags::NOT_NULL_FLAG),
            default_value: None,
            is_primary_key: c.flags().contains(ColumnFlags::PRI_KEY_FLAG),
            extra: String::new(),
        })
        .collect();

    let rows: Vec<Row> = result.iter().map(mysql_row_to_values).collect();

    Ok(QueryResult {
        columns,
        rows,
        affected_rows: 0,
        execution_time_ms,
    })
}

fn values_to_mysql_params(params: &[Value]) -> mysql_async::Params {
    if params.is_empty() {
        return mysql_async::Params::Empty;
    }
    let values: Vec<mysql_async::Value> = params.iter().map(value_to_mysql).collect();
    mysql_async::Params::Positional(values)
}

fn value_to_mysql(v: &Value) -> mysql_async::Value {
    match v {
        Value::Null => mysql_async::Value::NULL,
        Value::Bool(b) => mysql_async::Value::from(*b),
        Value::Int(i) => mysql_async::Value::from(*i),
        Value::Float(f) => mysql_async::Value::from(*f),
        Value::String(s) => mysql_async::Value::from(s.as_str()),
        Value::Bytes(b) => mysql_async::Value::from(b.as_slice()),
        Value::DateTime(dt) => mysql_async::Value::from(dt.format("%Y-%m-%d %H:%M:%S").to_string()),
    }
}

fn mysql_row_to_values(row: &mysql_async::Row) -> Row {
    (0..row.len())
        .map(|i| {
            if let Some(val) = row.as_ref(i) {
                match val {
                    mysql_async::Value::NULL => Value::Null,
                    mysql_async::Value::Int(n) => Value::Int(*n),
                    mysql_async::Value::UInt(n) => Value::Int(*n as i64),
                    mysql_async::Value::Float(f) => Value::Float(*f as f64),
                    mysql_async::Value::Double(f) => Value::Float(*f),
                    mysql_async::Value::Bytes(b) => match String::from_utf8(b.clone()) {
                        Ok(s) => Value::String(s),
                        Err(_) => Value::Bytes(b.clone()),
                    },
                    mysql_async::Value::Date(year, mon, day, hour, min, sec, _usec) => {
                        let s = format!(
                            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                            year, mon, day, hour, min, sec
                        );
                        Value::String(s)
                    }
                    mysql_async::Value::Time(neg, days, hours, mins, secs, _usec) => {
                        let total_hours = *days * 24 + *hours as u32;
                        let sign = if *neg { "-" } else { "" };
                        Value::String(format!(
                            "{}{:02}:{:02}:{:02}",
                            sign, total_hours, mins, secs
                        ))
                    }
                }
            } else {
                Value::Null
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::profile::{ConnectionProfile, DatabaseEngine};

    fn test_profile() -> ConnectionProfile {
        ConnectionProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            engine: DatabaseEngine::MySql,
            host: "127.0.0.1".to_string(),
            port: 3306,
            user: "root".to_string(),
            default_database: Some("querybox".to_string()),
            file_path: None,
        }
    }

    #[tokio::test]
    async fn test_connect_and_list_tables() {
        let profile = test_profile();
        let opts = MySqlDriver::opts_from_profile(&profile, "password");
        let pool = Pool::new(opts);
        let driver = MySqlDriver { pool: Some(pool) };

        let tables = driver.tables("querybox").await.unwrap();
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));

        driver.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_query() {
        let profile = test_profile();
        let opts = MySqlDriver::opts_from_profile(&profile, "password");
        let pool = Pool::new(opts);
        let driver = MySqlDriver { pool: Some(pool) };

        let result = driver
            .query("SELECT * FROM querybox.users", &[])
            .await
            .unwrap();
        assert!(!result.columns.is_empty());
        assert!(result.rows.len() >= 3);

        driver.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_foreign_keys() {
        let profile = test_profile();
        let opts = MySqlDriver::opts_from_profile(&profile, "password");
        let pool = Pool::new(opts);
        let driver = MySqlDriver { pool: Some(pool) };

        let fks = driver.foreign_keys("querybox", "orders").await.unwrap();
        assert!(
            fks.iter().any(|fk| fk.column == "user_id"),
            "orders should have a FK on user_id, got: {:?}",
            fks
        );

        driver.disconnect().await.unwrap();
    }
}
