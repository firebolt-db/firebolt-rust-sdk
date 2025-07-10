use crate::error::FireboltError;
use crate::result::ResultSet;
use std::collections::HashMap;

pub struct FireboltClient {
    _client_id: String,
    _client_secret: String,
    _token: Option<String>,
    _parameters: HashMap<String, String>,
    _engine_url: Option<String>,
    _api_endpoint: String,
}

impl FireboltClient {
    pub async fn query(&mut self, sql: &str) -> Result<ResultSet, FireboltError> {
        let engine_url = self
            .engine_url()
            .ok_or_else(|| FireboltError::Configuration("Engine URL not set".to_string()))?;

        let url = if engine_url.ends_with('/') {
            engine_url.to_string()
        } else {
            format!("{engine_url}/")
        };

        let mut params = self.parameters().clone();
        params.insert("output_format".to_string(), "JSON_Compact".to_string());

        self.execute_query_request(&url, sql, &params).await
    }

    async fn execute_query_request(
        &mut self,
        url: &str,
        sql: &str,
        params: &HashMap<String, String>,
    ) -> Result<ResultSet, FireboltError> {
        let client = reqwest::Client::new();
        let token = self
            ._token
            .as_ref()
            .ok_or_else(|| FireboltError::Authentication("No token available".to_string()))?;

        let response = client
            .post(url)
            .query(params)
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", crate::version::user_agent())
            .header(
                "Firebolt-Protocol-Version",
                crate::version::PROTOCOL_VERSION,
            )
            .body(sql.to_string())
            .send()
            .await
            .map_err(|e| FireboltError::Network(format!("Request failed: {e}")))?;

        let status = response.status();

        if status == 401 {
            self.refresh_token_and_retry(url, sql, params).await
        } else if status.is_server_error() {
            let body = response.text().await.map_err(|e| {
                FireboltError::Network(format!("Failed to read error response: {e}"))
            })?;
            Err(self.parse_server_error(body))
        } else if status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| FireboltError::Network(format!("Failed to read response: {e}")))?;
            self.parse_response(body)
        } else {
            let body = response.text().await.map_err(|e| {
                FireboltError::Network(format!("Failed to read error response: {e}"))
            })?;
            Err(self.parse_server_error(body))
        }
    }

    async fn refresh_token_and_retry(
        &mut self,
        url: &str,
        sql: &str,
        params: &HashMap<String, String>,
    ) -> Result<ResultSet, FireboltError> {
        let (new_token, _expiration) = crate::auth::authenticate(
            self.client_id().to_string(),
            self.client_secret().to_string(),
            self.api_endpoint().to_string(),
        )
        .await
        .map_err(|e| FireboltError::Authentication(format!("Token refresh failed: {e}")))?;

        self.set_token(new_token);

        let client = reqwest::Client::new();
        let token = self._token.as_ref().unwrap();

        let response = client
            .post(url)
            .query(params)
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", crate::version::user_agent())
            .header(
                "Firebolt-Protocol-Version",
                crate::version::PROTOCOL_VERSION,
            )
            .body(sql.to_string())
            .send()
            .await
            .map_err(|e| FireboltError::Network(format!("Retry request failed: {e}")))?;

        let status = response.status();

        if status.is_success() {
            let body = response.text().await.map_err(|e| {
                FireboltError::Network(format!("Failed to read retry response: {e}"))
            })?;
            self.parse_response(body)
        } else {
            let body = response.text().await.map_err(|e| {
                FireboltError::Network(format!("Failed to read retry error response: {e}"))
            })?;
            Err(FireboltError::Authentication(format!(
                "Authentication failed after token refresh: {body}"
            )))
        }
    }

    fn parse_response(&self, body: String) -> Result<ResultSet, FireboltError> {
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
            .map(|row_obj| {
                let row_values: Vec<serde_json::Value> = columns
                    .iter()
                    .map(|col| {
                        row_obj
                            .get(&col.name)
                            .cloned()
                            .unwrap_or(serde_json::Value::Null)
                    })
                    .collect();

                Ok(crate::result::Row::new(row_values))
            })
            .collect();

        let rows = rows?;

        Ok(ResultSet { columns, rows })
    }

    fn parse_server_error(&self, body: String) -> FireboltError {
        FireboltError::Query(format!("Server error: {body}"))
    }

    fn client_id(&self) -> &str {
        &self._client_id
    }

    fn client_secret(&self) -> &str {
        &self._client_secret
    }

    fn api_endpoint(&self) -> &str {
        &self._api_endpoint
    }

    fn engine_url(&self) -> Option<&str> {
        self._engine_url.as_deref()
    }

    fn parameters(&self) -> &HashMap<String, String> {
        &self._parameters
    }

    fn set_token(&mut self, token: String) {
        self._token = Some(token);
    }

    pub fn builder() -> FireboltClientFactory {
        FireboltClientFactory::new()
    }
}

pub struct FireboltClientFactory {
    client_id: Option<String>,
    client_secret: Option<String>,
    database_name: Option<String>,
    engine_name: Option<String>,
    account_name: Option<String>,
    _api_endpoint: String,
}

impl FireboltClientFactory {
    fn new() -> Self {
        Self {
            client_id: None,
            client_secret: None,
            database_name: None,
            engine_name: None,
            account_name: None,
            _api_endpoint: "https://api.firebolt.io".to_string(),
        }
    }

    pub fn with_credentials(mut self, client_id: String, client_secret: String) -> Self {
        self.client_id = Some(client_id);
        self.client_secret = Some(client_secret);
        self
    }

    pub fn with_database(mut self, database_name: String) -> Self {
        self.database_name = Some(database_name);
        self
    }

    pub fn with_engine(mut self, engine_name: String) -> Self {
        self.engine_name = Some(engine_name);
        self
    }

    pub fn with_account(mut self, account_name: String) -> Self {
        self.account_name = Some(account_name);
        self
    }

    pub async fn build(self) -> Result<FireboltClient, FireboltError> {
        todo!("FireboltClientFactory::build implementation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_response_success() {
        let client = create_test_client();
        let json_response = r#"{
            "meta": [
                {"name": "id", "type": "int"},
                {"name": "name", "type": "text"}
            ],
            "data": [
                {"id": 1, "name": "test"},
                {"id": 2, "name": "example"}
            ],
            "rows": 2,
            "statistics": {"elapsed": 0.006947, "rows_read": 2, "bytes_read": 10}
        }"#;

        let result = client.parse_response(json_response.to_string());
        assert!(result.is_ok());

        let result_set = result.unwrap();
        assert_eq!(result_set.columns.len(), 2);
        assert_eq!(result_set.rows.len(), 2);
        assert_eq!(result_set.columns[0].name, "id");
        assert_eq!(result_set.columns[1].name, "name");
    }

    #[tokio::test]
    async fn test_parse_response_invalid_json() {
        let client = create_test_client();
        let invalid_json = "invalid json";

        let result = client.parse_response(invalid_json.to_string());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Serialization(_)
        ));
    }

    #[tokio::test]
    async fn test_parse_response_missing_meta() {
        let client = create_test_client();
        let json_response = r#"{"data": []}"#;

        let result = client.parse_response(json_response.to_string());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FireboltError::Query(_)));
    }

    #[tokio::test]
    async fn test_parse_response_missing_data() {
        let client = create_test_client();
        let json_response = r#"{"meta": []}"#;

        let result = client.parse_response(json_response.to_string());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FireboltError::Query(_)));
    }

    #[test]
    fn test_client_getters() {
        let client = create_test_client();
        assert_eq!(client.client_id(), "test_id");
        assert_eq!(client.client_secret(), "test_secret");
        assert_eq!(client.api_endpoint(), "https://api.test.firebolt.io");
        assert_eq!(client.engine_url(), Some("https://test.engine.url/"));
        assert!(client.parameters().is_empty());
    }

    #[test]
    fn test_set_token() {
        let mut client = create_test_client();
        client.set_token("new_token".to_string());
        assert_eq!(client._token, Some("new_token".to_string()));
    }

    fn create_test_client() -> FireboltClient {
        FireboltClient {
            _client_id: "test_id".to_string(),
            _client_secret: "test_secret".to_string(),
            _token: Some("test_token".to_string()),
            _parameters: HashMap::new(),
            _engine_url: Some("https://test.engine.url/".to_string()),
            _api_endpoint: "https://api.test.firebolt.io".to_string(),
        }
    }
}
