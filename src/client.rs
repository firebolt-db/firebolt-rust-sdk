use crate::error::FireboltError;
use crate::result::ResultSet;
use std::collections::HashMap;
use url::Url;

const HEADER_UPDATE_ENDPOINT: &str = "Firebolt-Update-Endpoint";
const HEADER_UPDATE_PARAMETERS: &str = "Firebolt-Update-Parameters";
const HEADER_RESET_SESSION: &str = "Firebolt-Reset-Session";
const HEADER_REMOVE_PARAMETERS: &str = "Firebolt-Remove-Parameters";

#[derive(Debug)]
pub struct FireboltClient {
    _client_id: String,
    _client_secret: String,
    _token: String,
    _parameters: HashMap<String, String>,
    _engine_url: String,
    _api_endpoint: String,
}

impl FireboltClient {
    pub async fn query(&mut self, sql: &str) -> Result<ResultSet, FireboltError> {
        let engine_url = self.engine_url();
        let url = ensure_trailing_slash(engine_url);

        let mut params = self.parameters().clone();
        params.insert("output_format".to_string(), "JSON_Compact".to_string());

        self.execute_query_request(&url, sql, &params, true).await
    }

    async fn execute_query_request(
        &mut self,
        url: &str,
        sql: &str,
        params: &HashMap<String, String>,
        should_retry: bool,
    ) -> Result<ResultSet, FireboltError> {
        let client = reqwest::Client::new();
        let token = &self._token;

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

        if status == 401 && should_retry {
            let (new_token, _expiration) = crate::auth::authenticate(
                self.client_id().to_string(),
                self.client_secret().to_string(),
                self.api_endpoint().to_string(),
            )
            .await
            .map_err(|e| FireboltError::Authentication(format!("Token refresh failed: {e}")))?;

            self.set_token(new_token);
            Box::pin(self.execute_query_request(url, sql, params, false)).await
        } else if status == 401 {
            Err(FireboltError::Authentication(
                "Authentication failed after token refresh".to_string(),
            ))
        } else if status.is_server_error() {
            let body = response.text().await.map_err(|e| {
                FireboltError::Network(format!("Failed to read error response: {e}"))
            })?;
            Err(crate::parser::parse_server_error(body))
        } else if status.is_success() {
            self.process_response_headers(&response)?;
            let body = response
                .text()
                .await
                .map_err(|e| FireboltError::Network(format!("Failed to read response: {e}")))?;
            crate::parser::parse_response(body)
        } else {
            let body = response.text().await.map_err(|e| {
                FireboltError::Network(format!("Failed to read error response: {e}"))
            })?;
            Err(crate::parser::parse_server_error(body))
        }
    }

    pub fn client_id(&self) -> &str {
        &self._client_id
    }

    pub fn client_secret(&self) -> &str {
        &self._client_secret
    }

    pub fn api_endpoint(&self) -> &str {
        &self._api_endpoint
    }

    pub fn engine_url(&self) -> &str {
        &self._engine_url
    }

    pub fn parameters(&self) -> &HashMap<String, String> {
        &self._parameters
    }

    pub fn set_token(&mut self, token: String) {
        self._token = token;
    }

    pub fn builder() -> FireboltClientFactory {
        FireboltClientFactory::new()
    }

    fn process_response_headers(
        &mut self,
        response: &reqwest::Response,
    ) -> Result<(), FireboltError> {
        if let Some(endpoint_header) = response.headers().get(HEADER_UPDATE_ENDPOINT) {
            let endpoint_str = endpoint_header.to_str().map_err(|e| {
                FireboltError::HeaderParsing(format!("Invalid endpoint header: {e}"))
            })?;

            let url = Url::parse(FireboltClientFactory::fix_schema(endpoint_str).as_str())
                .map_err(|e| FireboltError::HeaderParsing(format!("Invalid endpoint URL: {e}")))?;

            let base_url = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
            let path = url.path();
            self._engine_url = if path == "/" || path.is_empty() {
                base_url
            } else {
                format!("{base_url}{path}")
            };

            for (key, value) in url.query_pairs() {
                self._parameters.insert(key.to_string(), value.to_string());
            }
        }

        if let Some(params_header) = response.headers().get(HEADER_UPDATE_PARAMETERS) {
            let params_str = params_header.to_str().map_err(|e| {
                FireboltError::HeaderParsing(format!("Invalid parameters header: {e}"))
            })?;

            for param_pair in params_str.split(',') {
                let param_pair = param_pair.trim();
                if param_pair.is_empty() {
                    continue;
                }

                let parts: Vec<&str> = param_pair.splitn(2, '=').collect();
                if parts.len() != 2 {
                    return Err(FireboltError::HeaderParsing(format!(
                        "Invalid parameter format: {param_pair}"
                    )));
                }

                let key = parts[0].trim();
                let value = parts[1].trim();

                if key.is_empty() {
                    return Err(FireboltError::HeaderParsing(
                        "Parameter key cannot be empty".to_string(),
                    ));
                }

                self._parameters.insert(key.to_string(), value.to_string());
            }
        }

        if response.headers().contains_key(HEADER_RESET_SESSION) {
            let database = self._parameters.get("database").cloned();
            let engine = self._parameters.get("engine").cloned();

            self._parameters.clear();

            if let Some(db) = database {
                self._parameters.insert("database".to_string(), db);
            }
            if let Some(eng) = engine {
                self._parameters.insert("engine".to_string(), eng);
            }
        }

        if let Some(remove_header) = response.headers().get(HEADER_REMOVE_PARAMETERS) {
            let remove_str = remove_header.to_str().map_err(|e| {
                FireboltError::HeaderParsing(format!("Invalid remove parameters header: {e}"))
            })?;

            for param_name in remove_str.split(',') {
                let param_name = param_name.trim();
                if !param_name.is_empty() {
                    self._parameters.remove(param_name);
                }
            }
        }

        Ok(())
    }
}

fn ensure_trailing_slash(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
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

    fn fix_schema(url: &str) -> String {
        if url.starts_with("https://") || url.starts_with("http://") {
            url.to_string()
        } else {
            format!("https://{url}")
        }
    }

    fn get_api_endpoint() -> String {
        let api_endpoint = std::env::var("FIREBOLT_API_ENDPOINT")
            .unwrap_or_else(|_| "api.app.firebolt.io".to_string());

        Self::fix_schema(&api_endpoint)
    }

    async fn get_engine_url(
        account_name: &str,
        api_endpoint: &str,
        token: &str,
    ) -> Result<String, FireboltError> {
        let engine_url_endpoint = format!("{api_endpoint}/web/v3/account/{account_name}/engineUrl");
        let client = reqwest::Client::new();

        let response = client
            .get(&engine_url_endpoint)
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", crate::version::user_agent())
            .send()
            .await
            .map_err(|e| FireboltError::Network(format!("Failed to get engine URL: {e}")))?;

        let status = response.status();

        match status.as_u16() {
            200 => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| FireboltError::Network(format!("Failed to read response: {e}")))?;

                let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
                    FireboltError::Query(format!("Failed to parse engine URL response: {e}"))
                })?;

                let engine_url =
                    json.get("engineUrl")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            FireboltError::Query("Missing engineUrl field in response".to_string())
                        })?;

                Ok(Self::fix_schema(ensure_trailing_slash(engine_url).as_str()))
            }
            404 => Err(FireboltError::Configuration(format!(
                "Account '{account_name}' not found"
            ))),
            _ => {
                let body = response.text().await.map_err(|e| {
                    FireboltError::Network(format!("Failed to read error response: {e}"))
                })?;
                Err(FireboltError::Query(body))
            }
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
        // 1. Validate required parameters
        let client_id = self
            .client_id
            .ok_or_else(|| FireboltError::Configuration("client_id is required".to_string()))?;
        let client_secret = self
            .client_secret
            .ok_or_else(|| FireboltError::Configuration("client_secret is required".to_string()))?;
        let account_name = self
            .account_name
            .ok_or_else(|| FireboltError::Configuration("account_name is required".to_string()))?;

        let api_endpoint = Self::get_api_endpoint();

        let (token, _expiration) = crate::auth::authenticate(
            client_id.clone(),
            client_secret.clone(),
            api_endpoint.clone(),
        )
        .await
        .map_err(FireboltError::Authentication)?;

        let engine_url = Self::get_engine_url(&account_name, &api_endpoint, &token).await?;

        let mut client = FireboltClient {
            _client_id: client_id,
            _client_secret: client_secret,
            _token: token,
            _parameters: HashMap::new(),
            _engine_url: engine_url,
            _api_endpoint: api_endpoint,
        };

        if let Some(database_name) = self.database_name {
            let use_database_sql = format!("USE DATABASE \"{database_name}\"");
            client.query(&use_database_sql).await.map_err(|e| {
                FireboltError::Configuration(format!("Failed to set database: {e}"))
            })?;
        }

        if let Some(engine_name) = self.engine_name {
            let use_engine_sql = format!("USE ENGINE \"{engine_name}\"");
            client
                .query(&use_engine_sql)
                .await
                .map_err(|e| FireboltError::Configuration(format!("Failed to set engine: {e}")))?;
        }

        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_query_request_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_request_retry_on_401() {
        let mut server = mockito::Server::new_async().await;

        let mock_401 = server
            .mock("POST", "/")
            .with_status(401)
            .expect(1)
            .create_async()
            .await;

        let _mock_success = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock_401.assert_async().await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Authentication(_)
        ));
    }

    #[tokio::test]
    async fn test_execute_query_request_no_retry_on_second_401() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(401)
            .expect(1)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), false)
            .await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Authentication(_)
        ));
    }

    #[tokio::test]
    async fn test_execute_query_request_5xx_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, FireboltError::Query(_)));
        assert!(format!("{error:?}").contains("Internal Server Error"));
    }

    #[test]
    fn test_client_getters() {
        let client = create_test_client();
        assert_eq!(client.client_id(), "test_id");
        assert_eq!(client.client_secret(), "test_secret");
        assert_eq!(client.api_endpoint(), "https://api.test.firebolt.io");
        assert_eq!(client.engine_url(), "https://test.engine.url/");
        assert!(client.parameters().is_empty());
    }

    #[test]
    fn test_set_token() {
        let mut client = create_test_client();
        client.set_token("new_token".to_string());
        assert_eq!(client._token, "new_token".to_string());
    }

    #[tokio::test]
    async fn test_execute_query_request_headers() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .match_header("User-Agent", crate::version::user_agent().as_str())
            .match_header(
                "Firebolt-Protocol-Version",
                crate::version::PROTOCOL_VERSION,
            )
            .match_header("Authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_trailing_slash() {
        assert_eq!(
            ensure_trailing_slash("https://example.com"),
            "https://example.com/"
        );
        assert_eq!(
            ensure_trailing_slash("https://example.com/"),
            "https://example.com/"
        );
        assert_eq!(ensure_trailing_slash(""), "/");
    }

    fn create_test_client() -> FireboltClient {
        FireboltClient {
            _client_id: "test_id".to_string(),
            _client_secret: "test_secret".to_string(),
            _token: "test_token".to_string(),
            _parameters: HashMap::new(),
            _engine_url: "https://test.engine.url/".to_string(),
            _api_endpoint: "https://api.test.firebolt.io".to_string(),
        }
    }

    #[tokio::test]
    async fn test_build_missing_client_id() {
        let mut server = mockito::Server::new_async().await;

        let _auth_mock = server
            .mock("POST", "/oauth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token": "test_token", "expires_in": 3600}"#)
            .create_async()
            .await;

        let _engine_mock = server
            .mock("GET", "/web/v3/account/test_account/engineUrl")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"engineUrl": "https://engine.test.firebolt.io/path"}"#)
            .create_async()
            .await;

        let api_endpoint = server
            .url()
            .replace("http://", "https://api.test.firebolt.io");
        std::env::set_var("FIREBOLT_API_ENDPOINT", &api_endpoint);

        let factory_no_id = FireboltClientFactory {
            client_id: None,
            client_secret: Some("secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: Some("test_account".to_string()),
            _api_endpoint: api_endpoint,
        };

        let result = factory_no_id.build().await;

        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Configuration(_)
        ));
    }

    #[tokio::test]
    async fn test_build_missing_client_secret() {
        let mut server = mockito::Server::new_async().await;

        let _auth_mock = server
            .mock("POST", "/oauth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token": "test_token", "expires_in": 3600}"#)
            .create_async()
            .await;

        let _engine_mock = server
            .mock("GET", "/web/v3/account/test_account/engineUrl")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"engineUrl": "https://engine.test.firebolt.io/path"}"#)
            .create_async()
            .await;

        let api_endpoint = server
            .url()
            .replace("http://", "https://api.test.firebolt.io");
        std::env::set_var("FIREBOLT_API_ENDPOINT", &api_endpoint);

        let factory_no_secret = FireboltClientFactory {
            client_id: Some("client_id".to_string()),
            client_secret: None,
            database_name: None,
            engine_name: None,
            account_name: Some("test_account".to_string()),
            _api_endpoint: api_endpoint,
        };

        let result = factory_no_secret.build().await;

        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Configuration(_)
        ));
    }

    #[tokio::test]
    async fn test_build_missing_account_name() {
        let mut server = mockito::Server::new_async().await;

        let _auth_mock = server
            .mock("POST", "/oauth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token": "test_token", "expires_in": 3600}"#)
            .create_async()
            .await;

        let _engine_mock = server
            .mock("GET", "/web/v3/account/test_account/engineUrl")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"engineUrl": "https://engine.test.firebolt.io/path"}"#)
            .create_async()
            .await;

        let api_endpoint = server
            .url()
            .replace("http://", "https://api.test.firebolt.io");
        std::env::set_var("FIREBOLT_API_ENDPOINT", &api_endpoint);

        let factory_no_account = FireboltClientFactory {
            client_id: Some("client_id".to_string()),
            client_secret: Some("secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: None,
            _api_endpoint: api_endpoint,
        };

        let result = factory_no_account.build().await;

        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Configuration(_)
        ));
    }

    #[tokio::test]
    async fn test_build_engine_url_success() {
        std::env::set_var("FIREBOLT_API_ENDPOINT", "api.test.firebolt.io");

        let factory = FireboltClientFactory {
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: Some("test_account".to_string()),
            _api_endpoint: "https://api.test.firebolt.io".to_string(),
        };

        let result = factory.build().await;

        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Authentication(_)
        ));
    }

    #[tokio::test]
    async fn test_build_account_not_found() {
        std::env::set_var("FIREBOLT_API_ENDPOINT", "api.test.firebolt.io");

        let factory = FireboltClientFactory {
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: Some("nonexistent_account".to_string()),
            _api_endpoint: "https://api.test.firebolt.io".to_string(),
        };

        let result = factory.build().await;

        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Authentication(_)
        ));
    }

    #[tokio::test]
    async fn test_build_server_error() {
        std::env::set_var("FIREBOLT_API_ENDPOINT", "api.test.firebolt.io");

        let factory = FireboltClientFactory {
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: Some("test_account".to_string()),
            _api_endpoint: "https://api.test.firebolt.io".to_string(),
        };

        let result = factory.build().await;

        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Authentication(_)
        ));
    }

    #[test]
    fn test_get_api_endpoint_default() {
        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        let result = FireboltClientFactory::get_api_endpoint();

        assert_eq!(result, "https://api.app.firebolt.io");
    }

    #[test]
    fn test_get_api_endpoint_from_env() {
        std::env::set_var("FIREBOLT_API_ENDPOINT", "custom.api.firebolt.io");

        let result = FireboltClientFactory::get_api_endpoint();

        assert_eq!(result, "https://custom.api.firebolt.io");

        std::env::remove_var("FIREBOLT_API_ENDPOINT");
    }

    #[test]
    fn test_get_api_endpoint_with_https_prefix() {
        std::env::set_var("FIREBOLT_API_ENDPOINT", "https://custom.api.firebolt.io");

        let result = FireboltClientFactory::get_api_endpoint();

        assert_eq!(result, "https://custom.api.firebolt.io");

        std::env::remove_var("FIREBOLT_API_ENDPOINT");
    }

    #[test]
    fn test_get_api_endpoint_with_http_prefix() {
        std::env::set_var("FIREBOLT_API_ENDPOINT", "http://custom.api.firebolt.io");

        let result = FireboltClientFactory::get_api_endpoint();

        assert_eq!(result, "http://custom.api.firebolt.io");

        std::env::remove_var("FIREBOLT_API_ENDPOINT");
    }

    #[tokio::test]
    async fn test_get_engine_url_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/web/v3/account/test_account/engineUrl")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"engineUrl": "engine.test.firebolt.io"}"#)
            .create_async()
            .await;

        let result =
            FireboltClientFactory::get_engine_url("test_account", &server.url(), "test_token")
                .await;

        mock.assert_async().await;
        assert!(result.is_ok());

        let engine_url = result.unwrap();
        assert_eq!(engine_url, "https://engine.test.firebolt.io/");
    }

    #[tokio::test]
    async fn test_get_engine_url_account_not_found() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/web/v3/account/nonexistent/engineUrl")
            .with_status(404)
            .create_async()
            .await;

        let result =
            FireboltClientFactory::get_engine_url("nonexistent", &server.url(), "test_token").await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Configuration(_)
        ));
    }

    #[tokio::test]
    async fn test_get_engine_url_server_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/web/v3/account/test_account/engineUrl")
            .with_status(500)
            .with_body("Internal server error")
            .create_async()
            .await;

        let result =
            FireboltClientFactory::get_engine_url("test_account", &server.url(), "test_token")
                .await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FireboltError::Query(_)));
    }

    #[tokio::test]
    async fn test_get_engine_url_invalid_json() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/web/v3/account/test_account/engineUrl")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("invalid json")
            .create_async()
            .await;

        let result =
            FireboltClientFactory::get_engine_url("test_account", &server.url(), "test_token")
                .await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FireboltError::Query(_)));
    }

    #[tokio::test]
    async fn test_get_engine_url_missing_engine_url_field() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/web/v3/account/test_account/engineUrl")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"otherField": "value"}"#)
            .create_async()
            .await;

        let result =
            FireboltClientFactory::get_engine_url("test_account", &server.url(), "test_token")
                .await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FireboltError::Query(_)));
    }

    #[tokio::test]
    async fn test_process_response_headers_update_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header(
                HEADER_UPDATE_ENDPOINT,
                "https://new.engine.url/path?param1=value1&param2=value2",
            )
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
        assert_eq!(client._engine_url, "https://new.engine.url/path");
        assert_eq!(
            client._parameters.get("param1"),
            Some(&"value1".to_string())
        );
        assert_eq!(
            client._parameters.get("param2"),
            Some(&"value2".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_response_headers_update_parameters() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header(
                HEADER_UPDATE_PARAMETERS,
                "database=new_db,engine=new_engine,custom=value",
            )
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
        assert_eq!(
            client._parameters.get("database"),
            Some(&"new_db".to_string())
        );
        assert_eq!(
            client._parameters.get("engine"),
            Some(&"new_engine".to_string())
        );
        assert_eq!(client._parameters.get("custom"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn test_process_response_headers_reset_session() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header(HEADER_RESET_SESSION, "true")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();
        client
            ._parameters
            .insert("database".to_string(), "test_db".to_string());
        client
            ._parameters
            .insert("engine".to_string(), "test_engine".to_string());
        client
            ._parameters
            .insert("custom_param".to_string(), "custom_value".to_string());

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
        assert_eq!(
            client._parameters.get("database"),
            Some(&"test_db".to_string())
        );
        assert_eq!(
            client._parameters.get("engine"),
            Some(&"test_engine".to_string())
        );
        assert_eq!(client._parameters.get("custom_param"), None);
        assert_eq!(client._parameters.len(), 2);
    }

    #[tokio::test]
    async fn test_process_response_headers_remove_parameters() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header(HEADER_REMOVE_PARAMETERS, "param1,param3")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();
        client
            ._parameters
            .insert("param1".to_string(), "value1".to_string());
        client
            ._parameters
            .insert("param2".to_string(), "value2".to_string());
        client
            ._parameters
            .insert("param3".to_string(), "value3".to_string());

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
        assert_eq!(client._parameters.get("param1"), None);
        assert_eq!(
            client._parameters.get("param2"),
            Some(&"value2".to_string())
        );
        assert_eq!(client._parameters.get("param3"), None);
        assert_eq!(client._parameters.len(), 1);
    }

    #[tokio::test]
    async fn test_process_response_headers_invalid_parameters_format() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header(HEADER_UPDATE_PARAMETERS, "invalid-format-no-equals")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::HeaderParsing(_)
        ));
    }

    #[tokio::test]
    async fn test_process_response_headers_empty_parameter_key() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header(HEADER_UPDATE_PARAMETERS, "=value")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_client();
        client._engine_url = server.url();

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FireboltError::HeaderParsing(_)
        ));
    }
}
