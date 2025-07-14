use crate::error::FireboltError;
use crate::result::ResultSet;
use crate::types::{Column, Type};
use regex::Regex;

fn parse_type(type_str: &str) -> Result<(Type, bool, Option<i32>, Option<i32>), FireboltError> {
    let is_nullable = type_str.starts_with("null::");
    let clean_type = if is_nullable {
        &type_str[6..]
    } else {
        type_str
    };

    if let Ok(decimal_regex) = Regex::new(r"decimal\((\d+),\s*(\d+)\)") {
        if let Some(captures) = decimal_regex.captures(clean_type) {
            let precision = captures[1]
                .parse()
                .map_err(|_| FireboltError::Query("Invalid decimal precision".to_string()))?;
            let scale = captures[2]
                .parse()
                .map_err(|_| FireboltError::Query("Invalid decimal scale".to_string()))?;
            return Ok((Type::Decimal, is_nullable, Some(precision), Some(scale)));
        }
    }

    if clean_type.starts_with("array") {
        return Ok((Type::Array, is_nullable, None, None));
    }

    let base_type = match clean_type {
        "int" => Type::Int,
        "bigint" | "long" => Type::Long,
        "float4" | "float" => Type::Float,
        "double" | "float8" => Type::Double,
        "decimal" => Type::Decimal,
        "text" | "string" => Type::Text,
        "date" => Type::Date,
        "timestamp" => Type::Timestamp,
        "timestamptz" => Type::TimestampTZ,
        "bool" | "boolean" => Type::Boolean,
        "bytea" => Type::Bytes,
        "geography" => Type::Geography,
        _ if clean_type.starts_with("struct") => Type::Struct,
        _ => {
            return Err(FireboltError::Query(format!(
                "Unsupported type: {clean_type}"
            )))
        }
    };

    Ok((base_type, is_nullable, None, None))
}

pub fn parse_columns(json: &serde_json::Value) -> Result<Vec<Column>, FireboltError> {
    let meta = json.get("meta").and_then(|m| m.as_array()).ok_or_else(|| {
        FireboltError::Query("Missing or invalid 'meta' field in response".to_string())
    })?;

    meta.iter()
        .map(|col| {
            let name = col
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| FireboltError::Query("Missing column name".to_string()))?
                .to_string();

            let type_str = col
                .get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| FireboltError::Query("Missing column type".to_string()))?;

            let (r#type, is_nullable, precision, scale) = parse_type(type_str)?;

            Ok(Column {
                name,
                r#type,
                precision,
                scale,
                is_nullable,
            })
        })
        .collect()
}

pub fn parse_data(
    json: &serde_json::Value,
    columns: &[Column],
) -> Result<Vec<crate::result::Row>, FireboltError> {
    let data = json.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
        FireboltError::Query("Missing or invalid 'data' field in response".to_string())
    })?;

    data.iter()
        .map(|row_array| {
            let row_values: Vec<serde_json::Value> = row_array
                .as_array()
                .ok_or_else(|| FireboltError::Query("Row data is not an array".to_string()))?
                .to_vec();

            Ok(crate::result::Row::new(row_values, columns.to_vec()))
        })
        .collect()
}

pub fn parse_response(body: String) -> Result<ResultSet, FireboltError> {
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| FireboltError::Serialization(format!("Failed to parse JSON: {e}")))?;

    let columns = parse_columns(&json)?;
    let rows = parse_data(&json, &columns)?;

    Ok(ResultSet { columns, rows })
}

pub fn parse_server_error(body: String) -> FireboltError {
    FireboltError::Query(format!("Server error: {body}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Type;

    #[test]
    fn test_parse_type_basic_types() {
        assert_eq!(parse_type("int").unwrap(), (Type::Int, false, None, None));
        assert_eq!(
            parse_type("bigint").unwrap(),
            (Type::Long, false, None, None)
        );
        assert_eq!(parse_type("long").unwrap(), (Type::Long, false, None, None));
        assert_eq!(
            parse_type("float").unwrap(),
            (Type::Float, false, None, None)
        );
        assert_eq!(
            parse_type("float4").unwrap(),
            (Type::Float, false, None, None)
        );
        assert_eq!(
            parse_type("double").unwrap(),
            (Type::Double, false, None, None)
        );
        assert_eq!(
            parse_type("float8").unwrap(),
            (Type::Double, false, None, None)
        );
        assert_eq!(parse_type("text").unwrap(), (Type::Text, false, None, None));
        assert_eq!(
            parse_type("string").unwrap(),
            (Type::Text, false, None, None)
        );
        assert_eq!(parse_type("date").unwrap(), (Type::Date, false, None, None));
        assert_eq!(
            parse_type("timestamp").unwrap(),
            (Type::Timestamp, false, None, None)
        );
        assert_eq!(
            parse_type("timestamptz").unwrap(),
            (Type::TimestampTZ, false, None, None)
        );
        assert_eq!(
            parse_type("bool").unwrap(),
            (Type::Boolean, false, None, None)
        );
        assert_eq!(
            parse_type("boolean").unwrap(),
            (Type::Boolean, false, None, None)
        );
        assert_eq!(
            parse_type("bytea").unwrap(),
            (Type::Bytes, false, None, None)
        );
        assert_eq!(
            parse_type("geography").unwrap(),
            (Type::Geography, false, None, None)
        );
        assert_eq!(
            parse_type("array(int)").unwrap(),
            (Type::Array, false, None, None)
        );
    }

    #[test]
    fn test_parse_type_nullable() {
        assert_eq!(
            parse_type("null::int").unwrap(),
            (Type::Int, true, None, None)
        );
        assert_eq!(
            parse_type("null::text").unwrap(),
            (Type::Text, true, None, None)
        );
        assert_eq!(
            parse_type("null::array(int)").unwrap(),
            (Type::Array, true, None, None)
        );
    }

    #[test]
    fn test_parse_type_decimal_with_precision_scale() {
        assert_eq!(
            parse_type("decimal(38, 30)").unwrap(),
            (Type::Decimal, false, Some(38), Some(30))
        );
        assert_eq!(
            parse_type("null::decimal(10, 2)").unwrap(),
            (Type::Decimal, true, Some(10), Some(2))
        );
    }

    #[test]
    fn test_parse_type_unsupported() {
        assert!(parse_type("unsupported_type").is_err());
    }

    #[test]
    fn test_parse_columns() {
        let json = serde_json::json!({
            "meta": [
                {"name": "id", "type": "int"},
                {"name": "name", "type": "text"},
                {"name": "price", "type": "decimal(10, 2)"},
                {"name": "nullable_field", "type": "null::text"}
            ]
        });

        let columns = parse_columns(&json).unwrap();
        assert_eq!(columns.len(), 4);

        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[0].r#type, Type::Int);
        assert!(!columns[0].is_nullable);

        assert_eq!(columns[1].name, "name");
        assert_eq!(columns[1].r#type, Type::Text);
        assert!(!columns[1].is_nullable);

        assert_eq!(columns[2].name, "price");
        assert_eq!(columns[2].r#type, Type::Decimal);
        assert_eq!(columns[2].precision, Some(10));
        assert_eq!(columns[2].scale, Some(2));
        assert!(!columns[2].is_nullable);

        assert_eq!(columns[3].name, "nullable_field");
        assert_eq!(columns[3].r#type, Type::Text);
        assert!(columns[3].is_nullable);
    }

    #[test]
    fn test_parse_data() {
        let columns = vec![
            Column {
                name: "id".to_string(),
                r#type: Type::Int,
                precision: None,
                scale: None,
                is_nullable: false,
            },
            Column {
                name: "name".to_string(),
                r#type: Type::Text,
                precision: None,
                scale: None,
                is_nullable: false,
            },
        ];

        let json = serde_json::json!({
            "data": [
                [1, "test"],
                [2, "example"]
            ]
        });

        let rows = parse_data(&json, &columns).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_parse_response_success() {
        let json_response = r#"{
            "meta": [
                {"name": "id", "type": "int"},
                {"name": "name", "type": "text"}
            ],
            "data": [
                [1, "test"],
                [2, "example"]
            ],
            "rows": 2,
            "statistics": {"elapsed": 0.006947, "rows_read": 2, "bytes_read": 10}
        }"#;

        let result = parse_response(json_response.to_string());
        assert!(result.is_ok());

        let result_set = result.unwrap();
        assert_eq!(result_set.columns.len(), 2);
        assert_eq!(result_set.rows.len(), 2);
        assert_eq!(result_set.columns[0].name, "id");
        assert_eq!(result_set.columns[0].r#type, Type::Int);
        assert_eq!(result_set.columns[1].name, "name");
        assert_eq!(result_set.columns[1].r#type, Type::Text);
    }

    #[test]
    fn test_parse_response_invalid_json() {
        let invalid_json = "invalid json";

        let result = parse_response(invalid_json.to_string());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Serialization(_)
        ));
    }

    #[test]
    fn test_parse_response_missing_meta() {
        let json_response = r#"{"data": []}"#;

        let result = parse_response(json_response.to_string());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FireboltError::Query(_)));
    }

    #[test]
    fn test_parse_response_missing_data() {
        let json_response = r#"{"meta": []}"#;

        let result = parse_response(json_response.to_string());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FireboltError::Query(_)));
    }

    #[test]
    fn test_parse_server_error() {
        let error_body = "Internal Server Error".to_string();
        let result = parse_server_error(error_body);
        assert!(matches!(result, FireboltError::Query(_)));
        assert!(format!("{result:?}").contains("Server error: Internal Server Error"));
    }
}
