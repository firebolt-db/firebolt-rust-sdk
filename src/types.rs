use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Long,
    Float,
    Double,
    Decimal,
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

pub trait TypeConversion {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError>
    where
        Self: Sized;
}

impl TypeConversion for i32 {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Int => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                value
                    .as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .ok_or_else(|| {
                        crate::error::FireboltError::Serialization(
                            "Failed to convert to i32".to_string(),
                        )
                    })
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to i32"
            ))),
        }
    }
}

impl TypeConversion for Option<i32> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Int => {
                let val = value
                    .as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .ok_or_else(|| {
                        crate::error::FireboltError::Serialization(
                            "Failed to convert to i32".to_string(),
                        )
                    })?;
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<i32>"
            ))),
        }
    }
}

impl TypeConversion for num_bigint::BigInt {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Long => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                if let Some(v) = value.as_i64() {
                    Ok(num_bigint::BigInt::from(v))
                } else if let Some(s) = value.as_str() {
                    s.parse().map_err(|_| {
                        crate::error::FireboltError::Serialization(
                            "Failed to parse BigInt from string".to_string(),
                        )
                    })
                } else {
                    Err(crate::error::FireboltError::Serialization(
                        "Failed to convert to BigInt".to_string(),
                    ))
                }
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to BigInt"
            ))),
        }
    }
}

impl TypeConversion for Option<num_bigint::BigInt> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Long => {
                let val = if let Some(v) = value.as_i64() {
                    num_bigint::BigInt::from(v)
                } else if let Some(s) = value.as_str() {
                    s.parse().map_err(|_| {
                        crate::error::FireboltError::Serialization(
                            "Failed to parse BigInt from string".to_string(),
                        )
                    })?
                } else {
                    return Err(crate::error::FireboltError::Serialization(
                        "Failed to convert to BigInt".to_string(),
                    ));
                };
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<BigInt>"
            ))),
        }
    }
}

impl TypeConversion for f32 {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Float => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                value.as_f64().map(|v| v as f32).ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to f32".to_string(),
                    )
                })
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to f32"
            ))),
        }
    }
}

impl TypeConversion for Option<f32> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Float => {
                let val = value.as_f64().map(|v| v as f32).ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to f32".to_string(),
                    )
                })?;
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<f32>"
            ))),
        }
    }
}

impl TypeConversion for f64 {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Double => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                value.as_f64().ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to f64".to_string(),
                    )
                })
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to f64"
            ))),
        }
    }
}

impl TypeConversion for Option<f64> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Double => {
                let val = value.as_f64().ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to f64".to_string(),
                    )
                })?;
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<f64>"
            ))),
        }
    }
}

impl TypeConversion for rust_decimal::Decimal {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Decimal => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                if let Some(s) = value.as_str() {
                    s.parse().map_err(|_| {
                        crate::error::FireboltError::Serialization(
                            "Failed to parse Decimal from string".to_string(),
                        )
                    })
                } else if let Some(f) = value.as_f64() {
                    rust_decimal::Decimal::try_from(f).map_err(|_| {
                        crate::error::FireboltError::Serialization(
                            "Failed to convert f64 to Decimal".to_string(),
                        )
                    })
                } else {
                    Err(crate::error::FireboltError::Serialization(
                        "Failed to convert to Decimal".to_string(),
                    ))
                }
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Decimal"
            ))),
        }
    }
}

impl TypeConversion for Option<rust_decimal::Decimal> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Decimal => {
                let val = if let Some(s) = value.as_str() {
                    s.parse().map_err(|_| {
                        crate::error::FireboltError::Serialization(
                            "Failed to parse Decimal from string".to_string(),
                        )
                    })?
                } else if let Some(f) = value.as_f64() {
                    rust_decimal::Decimal::try_from(f).map_err(|_| {
                        crate::error::FireboltError::Serialization(
                            "Failed to convert f64 to Decimal".to_string(),
                        )
                    })?
                } else {
                    return Err(crate::error::FireboltError::Serialization(
                        "Failed to convert to Decimal".to_string(),
                    ));
                };
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<Decimal>"
            ))),
        }
    }
}

impl TypeConversion for String {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Text => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                value.as_str().map(|s| s.to_string()).ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to String".to_string(),
                    )
                })
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to String"
            ))),
        }
    }
}

impl TypeConversion for Option<String> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Text => {
                let val = value.as_str().map(|s| s.to_string()).ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to String".to_string(),
                    )
                })?;
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<String>"
            ))),
        }
    }
}

impl TypeConversion for bool {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Boolean => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                value.as_bool().ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to bool".to_string(),
                    )
                })
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to bool"
            ))),
        }
    }
}

impl TypeConversion for Option<bool> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Boolean => {
                let val = value.as_bool().ok_or_else(|| {
                    crate::error::FireboltError::Serialization(
                        "Failed to convert to bool".to_string(),
                    )
                })?;
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<bool>"
            ))),
        }
    }
}

impl TypeConversion for Vec<u8> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        match column_type {
            Type::Bytes => {
                if value.is_null() {
                    return Err(crate::error::FireboltError::Serialization(
                        "Cannot convert null to non-nullable type".to_string(),
                    ));
                }
                if let Some(s) = value.as_str() {
                    if let Some(stripped) = s.strip_prefix("\\x") {
                        hex::decode(stripped).map_err(|_| {
                            crate::error::FireboltError::Serialization(
                                "Failed to decode hex string".to_string(),
                            )
                        })
                    } else {
                        Ok(s.as_bytes().to_vec())
                    }
                } else {
                    Err(crate::error::FireboltError::Serialization(
                        "Failed to convert to Vec<u8>".to_string(),
                    ))
                }
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Vec<u8>"
            ))),
        }
    }
}

impl TypeConversion for Option<Vec<u8>> {
    fn convert_from_json(
        value: &serde_json::Value,
        column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        if value.is_null() {
            return Ok(None);
        }
        match column_type {
            Type::Bytes => {
                let val = if let Some(s) = value.as_str() {
                    if let Some(stripped) = s.strip_prefix("\\x") {
                        hex::decode(stripped).map_err(|_| {
                            crate::error::FireboltError::Serialization(
                                "Failed to decode hex string".to_string(),
                            )
                        })?
                    } else {
                        s.as_bytes().to_vec()
                    }
                } else {
                    return Err(crate::error::FireboltError::Serialization(
                        "Failed to convert to Vec<u8>".to_string(),
                    ));
                };
                Ok(Some(val))
            }
            _ => Err(crate::error::FireboltError::Serialization(format!(
                "Cannot convert {column_type:?} to Option<Vec<u8>>"
            ))),
        }
    }
}

impl TypeConversion for serde_json::Value {
    fn convert_from_json(
        value: &serde_json::Value,
        _column_type: &Type,
    ) -> Result<Self, crate::error::FireboltError> {
        Ok(value.clone())
    }
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
