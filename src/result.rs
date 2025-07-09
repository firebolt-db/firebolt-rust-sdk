use crate::error::FireboltError;
use crate::types::{Column, ColumnRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    data: Vec<serde_json::Value>,
}

impl Row {
    pub fn get<T>(&self, _column_ref: impl Into<ColumnRef>) -> Result<T, FireboltError>
    where
        T: serde::de::DeserializeOwned,
    {
        todo!("Row::get implementation")
    }
}
