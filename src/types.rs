use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Long,
    Float,
    Double,
    Text,
    Date,
    Timestamp,
    TimestampTZ,
    Boolean,
    Array,
    Struct,
    Geography,
    Bytes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub r#type: Type,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
    pub is_nullable: bool,
}

#[derive(Debug, Clone)]
pub enum ColumnRef {
    Index(usize),
    Name(String),
}

impl From<usize> for ColumnRef {
    fn from(index: usize) -> Self {
        ColumnRef::Index(index)
    }
}

impl From<String> for ColumnRef {
    fn from(name: String) -> Self {
        ColumnRef::Name(name)
    }
}

impl From<&str> for ColumnRef {
    fn from(name: &str) -> Self {
        ColumnRef::Name(name.to_string())
    }
}
