#![allow(dead_code)]

pub mod csv_export;
pub mod json_export;
pub mod sql_export;

use crate::db::types::QueryResult;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Sql,
    Json,
}

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn export_to_string(
    result: &QueryResult,
    format: ExportFormat,
    table_name: Option<&str>,
) -> Result<String, ExportError> {
    match format {
        ExportFormat::Csv => csv_export::export(result),
        ExportFormat::Sql => Ok(sql_export::export(result, table_name.unwrap_or("table"))),
        ExportFormat::Json => json_export::export(result),
    }
}
