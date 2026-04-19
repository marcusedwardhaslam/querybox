pub mod mysql;
pub mod postgres;
pub mod sqlite;
pub mod types;

use async_trait::async_trait;
use thiserror::Error;
use types::*;

use crate::connection::profile::ConnectionProfile;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Query failed: {0}")]
    Query(String),
    #[error("Not connected")]
    NotConnected,
    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    async fn connect(profile: &ConnectionProfile, password: &str) -> Result<Self, DbError>
    where
        Self: Sized;

    #[allow(dead_code)]
    async fn disconnect(&self) -> Result<(), DbError>;

    async fn databases(&self) -> Result<Vec<String>, DbError>;

    async fn tables(&self, database: &str) -> Result<Vec<String>, DbError>;

    #[allow(dead_code)]
    async fn columns(&self, database: &str, table: &str) -> Result<Vec<Column>, DbError>;

    #[allow(dead_code)]
    async fn indexes(&self, database: &str, table: &str) -> Result<Vec<Index>, DbError>;

    #[allow(dead_code)]
    async fn foreign_keys(&self, database: &str, table: &str) -> Result<Vec<ForeignKey>, DbError> {
        let _ = (database, table);
        Ok(vec![])
    }

    async fn query(&self, sql: &str, params: &[Value]) -> Result<QueryResult, DbError>;

    async fn query_in(
        &self,
        database: Option<&str>,
        sql: &str,
        params: &[Value],
    ) -> Result<QueryResult, DbError> {
        let _ = database;
        self.query(sql, params).await
    }

    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, DbError>;

    fn dialect(&self) -> Dialect;
}
