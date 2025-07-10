use crate::error::FireboltError;
use crate::result::ResultSet;
use std::collections::HashMap;

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

    fn client_id(&self) -> &str {
        &self._client_id
    }

    fn client_secret(&self) -> &str {
        &self._client_secret
    }

    fn api_endpoint(&self) -> &str {
        &self._api_endpoint
    }

    fn engine_url(&self) -> &str {
        &self._engine_url
    }

    fn parameters(&self) -> &HashMap<String, String> {
        &self._parameters
    }

    pub fn set_token(&mut self, token: String) {
        self._token = token;
    }

    pub fn builder() -> FireboltClientFactory {
        FireboltClientFactory::new()
    }

    #[doc(hidden)]
    pub fn new_for_testing(
        client_id: String,
        client_secret: String,
        token: String,
        engine_url: String,
        api_endpoint: String,
    ) -> Self {
        Self {
            _client_id: client_id,
            _client_secret: client_secret,
            _token: token,
            _parameters: std::collections::HashMap::new(),
            _engine_url: engine_url,
            _api_endpoint: api_endpoint,
        }
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
    async fn test_execute_query_request_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [{"test": 1}]}"#)
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
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [{"test": 1}]}"#)
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
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [{"test": 1}]}"#)
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
}
