use crate::error::FireboltError;
use crate::types::{Column, ColumnRef, TypeConversion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    data: Vec<serde_json::Value>,
    columns: Vec<Column>,
}

impl Row {
    pub fn new(data: Vec<serde_json::Value>, columns: Vec<Column>) -> Self {
        Self { data, columns }
    }

    pub fn get<T>(&self, column_ref: impl Into<ColumnRef>) -> Result<T, FireboltError>
    where
        T: TypeConversion,
    {
        let column_ref = column_ref.into();
        let (index, column) = match column_ref {
            ColumnRef::Index(i) => {
                let column = self.columns.get(i).ok_or_else(|| {
                    FireboltError::Query(format!("Column index {i} out of bounds"))
                })?;
                (i, column)
            }
            ColumnRef::Name(name) => {
                let (index, column) = self
                    .columns
                    .iter()
                    .enumerate()
                    .find(|(_, col)| col.name == name)
                    .ok_or_else(|| FireboltError::Query(format!("Column '{name}' not found")))?;
                (index, column)
            }
        };

        let value = self
            .data
            .get(index)
            .ok_or_else(|| FireboltError::Query(format!("Column index {index} out of bounds")))?;

        T::convert_from_json(value, &column.r#type)
    }
}
