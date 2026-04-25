use async_trait::async_trait;
use std::time::Instant;
use tokio_postgres::{Client, NoTls};

use super::types::*;
use super::{DatabaseDriver, DbError};
use crate::connection::profile::ConnectionProfile;

pub struct PostgresDriver {
    client: Client,
    _connection_handle: tokio::task::JoinHandle<()>,
}

#[async_trait]
impl DatabaseDriver for PostgresDriver {
    async fn connect(profile: &ConnectionProfile, password: &str) -> Result<Self, DbError> {
        let conn_str = format!(
            "host={} port={} user={} password={} dbname={}",
            profile.host,
            profile.port,
            profile.user,
            password,
            profile.default_database.as_deref().unwrap_or("postgres")
        );

        let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        Ok(Self {
            client,
            _connection_handle: handle,
        })
    }

    async fn disconnect(&self) -> Result<(), DbError> {
        Ok(())
    }

    async fn databases(&self) -> Result<Vec<String>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
                &[],
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    async fn tables(&self, _database: &str) -> Result<Vec<String>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT table_name FROM information_schema.tables \
                 WHERE table_schema = 'public' ORDER BY table_name",
                &[],
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    async fn columns(&self, _database: &str, table: &str) -> Result<Vec<Column>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default, \
                 CASE WHEN tc.constraint_type = 'PRIMARY KEY' THEN 'PRI' ELSE '' END as key_type \
                 FROM information_schema.columns c \
                 LEFT JOIN information_schema.key_column_usage kcu \
                   ON c.column_name = kcu.column_name AND c.table_name = kcu.table_name \
                 LEFT JOIN information_schema.table_constraints tc \
                   ON kcu.constraint_name = tc.constraint_name AND tc.constraint_type = 'PRIMARY KEY' \
                 WHERE c.table_name = $1 AND c.table_schema = 'public' \
                 ORDER BY c.ordinal_position",
                &[&table],
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| {
                let key: String = r.get(4);
                Column {
                    name: r.get(0),
                    data_type: r.get(1),
                    nullable: r.get::<_, String>(2) == "YES",
                    default_value: r.get(3),
                    is_primary_key: key == "PRI",
                    extra: String::new(),
                }
            })
            .collect())
    }

    async fn indexes(&self, _database: &str, table: &str) -> Result<Vec<Index>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT i.relname as index_name, a.attname as column_name, ix.indisunique \
                 FROM pg_class t, pg_class i, pg_index ix, pg_attribute a \
                 WHERE t.oid = ix.indrelid AND i.oid = ix.indexrelid AND a.attrelid = t.oid \
                 AND a.attnum = ANY(ix.indkey) AND t.relkind = 'r' AND t.relname = $1 \
                 ORDER BY i.relname, a.attnum",
                &[&table],
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let mut indexes: Vec<Index> = vec![];
        for row in &rows {
            let name: String = row.get(0);
            let column: String = row.get(1);
            let unique: bool = row.get(2);

            if let Some(idx) = indexes.iter_mut().find(|i| i.name == name) {
                idx.columns.push(column);
            } else {
                indexes.push(Index {
                    name,
                    columns: vec![column],
                    unique,
                });
            }
        }
        Ok(indexes)
    }

    async fn query(&self, sql: &str, params: &[Value]) -> Result<QueryResult, DbError> {
        let start = Instant::now();
        let pg_params = to_pg_params(params);
        let pg_params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = pg_params
            .iter()
            .map(|p| -> &(dyn tokio_postgres::types::ToSql + Sync) { p.as_ref() })
            .collect();
        let rows = self
            .client
            .query(sql, &pg_params_refs)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        if rows.is_empty() {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: 0,
                execution_time_ms,
            });
        }

        let columns: Vec<Column> = rows[0]
            .columns()
            .iter()
            .map(|c| Column {
                name: c.name().to_string(),
                data_type: format!("{:?}", c.type_()),
                nullable: true,
                default_value: None,
                is_primary_key: false,
                extra: String::new(),
            })
            .collect();

        let result_rows: Vec<Row> = rows.iter().map(pg_row_to_values).collect();

        Ok(QueryResult {
            columns,
            rows: result_rows,
            affected_rows: 0,
            execution_time_ms,
        })
    }

    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, DbError> {
        let pg_params = to_pg_params(params);
        let pg_params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = pg_params
            .iter()
            .map(|p| -> &(dyn tokio_postgres::types::ToSql + Sync) { p.as_ref() })
            .collect();
        let result = self
            .client
            .execute(sql, &pg_params_refs)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(result)
    }

    fn dialect(&self) -> Dialect {
        Dialect::PostgreSql
    }
}

fn to_pg_params(params: &[Value]) -> Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> {
    params
        .iter()
        .map(|v| -> Box<dyn tokio_postgres::types::ToSql + Sync + Send> {
            match v {
                Value::Null => Box::new(Option::<String>::None),
                Value::Bool(b) => Box::new(*b),
                Value::Int(i) => Box::new(*i),
                Value::Float(f) => Box::new(*f),
                Value::String(s) => Box::new(s.clone()),
                Value::Bytes(b) => Box::new(b.clone()),
                Value::DateTime(dt) => Box::new(dt.to_string()),
                Value::RawSql(_) => unreachable!("RawSql is consumed before reaching the driver"),
            }
        })
        .collect()
}

fn pg_row_to_values(row: &tokio_postgres::Row) -> Row {
    let mut values = vec![];
    for i in 0..row.len() {
        let col_type = row.columns()[i].type_();
        let val = match col_type.name() {
            "int2" | "int4" => row
                .try_get::<_, i32>(i)
                .ok()
                .map(|v| Value::Int(v as i64))
                .unwrap_or(Value::Null),
            "int8" => row
                .try_get::<_, i64>(i)
                .ok()
                .map(Value::Int)
                .unwrap_or(Value::Null),
            "float4" => row
                .try_get::<_, f32>(i)
                .ok()
                .map(|v| Value::Float(v as f64))
                .unwrap_or(Value::Null),
            "float8" => row
                .try_get::<_, f64>(i)
                .ok()
                .map(Value::Float)
                .unwrap_or(Value::Null),
            "bool" => row
                .try_get::<_, bool>(i)
                .ok()
                .map(Value::Bool)
                .unwrap_or(Value::Null),
            "text" | "varchar" | "name" | "bpchar" => row
                .try_get::<_, String>(i)
                .ok()
                .map(Value::String)
                .unwrap_or(Value::Null),
            _ => row
                .try_get::<_, String>(i)
                .ok()
                .map(Value::String)
                .unwrap_or(Value::Null),
        };
        values.push(val);
    }
    values
}
