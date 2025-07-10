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
    pub fn new(data: Vec<serde_json::Value>) -> Self {
        Self { data }
    }

    pub fn get<T>(&self, column_ref: impl Into<ColumnRef>) -> Result<T, FireboltError>
    where
        T: serde::de::DeserializeOwned,
    {
        let column_ref = column_ref.into();
        let index = match column_ref {
            ColumnRef::Index(i) => i,
            ColumnRef::Name(_) => {
                return Err(FireboltError::Query(
                    "Column name lookup not implemented".to_string(),
                ))
            }
        };

        let value = self.data.get(index).ok_or_else(|| {
            FireboltError::Query(format!("Column index {index} out of bounds"))
        })?;

        serde_json::from_value(value.clone())
            .map_err(|e| FireboltError::Serialization(format!("Failed to deserialize column value: {e}")))
    }
}
