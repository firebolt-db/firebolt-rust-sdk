use crate::error::FireboltError;
use crate::result::ResultSet;

pub fn parse_response(body: String) -> Result<ResultSet, FireboltError> {
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| FireboltError::Serialization(format!("Failed to parse JSON: {e}")))?;

    let meta = json.get("meta").and_then(|m| m.as_array()).ok_or_else(|| {
        FireboltError::Query("Missing or invalid 'meta' field in response".to_string())
    })?;

    let data = json.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
        FireboltError::Query("Missing or invalid 'data' field in response".to_string())
    })?;

    let columns: Result<Vec<crate::types::Column>, FireboltError> = meta
        .iter()
        .map(|col| {
            let name = col
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| FireboltError::Query("Missing column name".to_string()))?
                .to_string();

            Ok(crate::types::Column {
                name,
                r#type: crate::types::Type::Text,
                precision: None,
                scale: None,
                is_nullable: false,
            })
        })
        .collect();

    let columns = columns?;

    let rows: Result<Vec<crate::result::Row>, FireboltError> = data
        .iter()
        .map(|row_array| {
            let row_values: Vec<serde_json::Value> = row_array
                .as_array()
                .ok_or_else(|| FireboltError::Query("Row data is not an array".to_string()))?
                .to_vec();

            Ok(crate::result::Row::new(row_values))
        })
        .collect();

    let rows = rows?;

    Ok(ResultSet { columns, rows })
}

pub fn parse_server_error(body: String) -> FireboltError {
    FireboltError::Query(format!("Server error: {body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(result_set.columns[1].name, "name");
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
