use crate::error::FireboltError;
use crate::result::ResultSet;
use std::collections::HashMap;
use url::Url;

const HEADER_UPDATE_ENDPOINT: &str = "Firebolt-Update-Endpoint";
const HEADER_UPDATE_PARAMETERS: &str = "Firebolt-Update-Parameters";
const HEADER_RESET_SESSION: &str = "Firebolt-Reset-Session";
const HEADER_REMOVE_PARAMETERS: &str = "Firebolt-Remove-Parameters";

#[derive(Debug)]
struct CloudAuth {
    client_id: String,
    client_secret: String,
    api_endpoint: String,
    token: String,
}

#[derive(Debug)]
pub struct FireboltClient {
    _auth: Option<CloudAuth>,
    _parameters: HashMap<String, String>,
    _engine_url: String,
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

        let mut request = client
            .post(url)
            .query(params)
            .header("User-Agent", crate::version::user_agent())
            .header(
                "Firebolt-Protocol-Version",
                crate::version::PROTOCOL_VERSION,
            )
            .body(sql.to_string());

        if let Some(auth) = &self._auth {
            let token = &auth.token;
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send()
            .await
            .map_err(|e| FireboltError::Network(format!("Request failed: {e}")))?;

        let status = response.status();

        if status == 401 {
            let refresh = if should_retry {
                self._auth.as_ref().map(|auth| {
                    (
                        auth.client_id.clone(),
                        auth.client_secret.clone(),
                        auth.api_endpoint.clone(),
                    )
                })
            } else {
                None
            };

            if let Some((client_id, client_secret, api_endpoint)) = refresh {
                let (new_token, _expiration) =
                    crate::auth::authenticate(client_id, client_secret, api_endpoint)
                        .await
                        .map_err(|e| {
                            FireboltError::Authentication(format!("Token refresh failed: {e}"))
                        })?;

                self.set_token(new_token)?;
                return Box::pin(self.execute_query_request(url, sql, params, false)).await;
            }

            let body = response.text().await.map_err(|e| {
                FireboltError::Network(format!("Failed to read error response: {e}"))
            })?;

            return Err(FireboltError::Authentication(if body.trim().is_empty() {
                "Authentication failed".to_string()
            } else {
                body
            }));
        }

        if status.is_server_error() {
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

    pub fn client_id(&self) -> Option<&str> {
        self._auth.as_ref().map(|auth| auth.client_id.as_str())
    }

    pub fn client_secret(&self) -> Option<&str> {
        self._auth.as_ref().map(|auth| auth.client_secret.as_str())
    }

    pub fn api_endpoint(&self) -> Option<&str> {
        self._auth.as_ref().map(|auth| auth.api_endpoint.as_str())
    }

    pub fn engine_url(&self) -> &str {
        &self._engine_url
    }

    pub fn parameters(&self) -> &HashMap<String, String> {
        &self._parameters
    }

    pub fn set_token(&mut self, token: String) -> Result<(), FireboltError> {
        match self._auth.as_mut() {
            Some(auth) => {
                auth.token = token;
                Ok(())
            }
            None => Err(FireboltError::Configuration(
                "set_token is not applicable to a Firebolt Core connection, which has no authentication".to_string(),
            )),
        }
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
    url: Option<String>,
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
            url: None,
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

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub async fn build(mut self) -> Result<FireboltClient, FireboltError> {
        match self.url.take() {
            Some(url) => self.build_core(url).await,
            None => self.build_cloud().await,
        }
    }

    async fn build_core(self, url: String) -> Result<FireboltClient, FireboltError> {
        if self.client_id.is_some() || self.client_secret.is_some() {
            return Err(FireboltError::Configuration(
                "client_id and client_secret cannot be combined with url: Firebolt Core has no authentication".to_string(),
            ));
        }

        if self.account_name.is_some() {
            return Err(FireboltError::Configuration(
                "account_name cannot be combined with url: Firebolt Core has no accounts"
                    .to_string(),
            ));
        }

        if self.engine_name.is_some() {
            return Err(FireboltError::Configuration(
                "engine cannot be combined with url: Firebolt Core has no engines".to_string(),
            ));
        }

        Self::validate_core_url(&url)?;

        let mut client = FireboltClient {
            _auth: None,
            _parameters: HashMap::new(),
            _engine_url: url,
        };

        if let Some(database_name) = self.database_name {
            let use_database_sql = format!("USE DATABASE \"{database_name}\"");
            client.query(&use_database_sql).await.map_err(|e| {
                FireboltError::Configuration(format!("Failed to set database: {e}"))
            })?;
        }

        Ok(client)
    }

    fn validate_core_url(url: &str) -> Result<(), FireboltError> {
        let parsed = Url::parse(url)
            .map_err(|e| FireboltError::Configuration(format!("Invalid url '{url}': {e}")))?;

        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(FireboltError::Configuration(format!(
                "Invalid url '{url}': scheme must be http or https"
            )));
        }

        let host_missing = match parsed.host_str() {
            Some(host) => host.is_empty(),
            None => true,
        };
        if host_missing {
            return Err(FireboltError::Configuration(format!(
                "Invalid url '{url}': missing host"
            )));
        }

        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(FireboltError::Configuration(format!(
                "Invalid url '{url}': credentials in the url are not supported, Firebolt Core has no authentication"
            )));
        }

        if parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(FireboltError::Configuration(format!(
                "Invalid url '{url}': a query string or fragment is not supported"
            )));
        }

        Ok(())
    }

    async fn build_cloud(self) -> Result<FireboltClient, FireboltError> {
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
            _auth: Some(CloudAuth {
                client_id,
                client_secret,
                api_endpoint,
                token,
            }),
            _parameters: HashMap::new(),
            _engine_url: engine_url,
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
    use mockito::Matcher;

    fn env_lock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    fn create_test_core_client(engine_url: String) -> FireboltClient {
        FireboltClient {
            _auth: None,
            _parameters: HashMap::new(),
            _engine_url: engine_url,
        }
    }

    #[tokio::test]
    async fn test_execute_query_request_omits_authorization_without_auth() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .match_header("Authorization", Matcher::Missing)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = create_test_core_client(server.url());

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_request_401_without_auth_is_not_retried() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(401)
            .with_body("core says no")
            .expect(1)
            .create_async()
            .await;

        let mut client = create_test_core_client(server.url());

        let result = client
            .execute_query_request(&server.url(), "SELECT 1", &HashMap::new(), true)
            .await;

        mock.assert_async().await;
        let error = result.unwrap_err();
        assert!(matches!(error, FireboltError::Authentication(_)));
        assert!(format!("{error}").contains("core says no"));
    }

    #[tokio::test]
    async fn test_build_core_no_credentials_succeeds() {
        let server = mockito::Server::new_async().await;

        let client = FireboltClient::builder()
            .with_url(server.url())
            .build()
            .await
            .expect("core build should succeed");

        assert_eq!(client.client_id(), None);
        assert_eq!(client.client_secret(), None);
        assert_eq!(client.api_endpoint(), None);
        assert_eq!(client.engine_url(), server.url());
    }

    #[tokio::test]
    async fn test_build_core_with_database_issues_use_database() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .match_query(Matcher::Any)
            .match_body("USE DATABASE \"my_db\"")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .expect(1)
            .create_async()
            .await;

        let result = FireboltClient::builder()
            .with_url(server.url())
            .with_database("my_db".to_string())
            .build()
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_core_query_omits_authorization_header() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .match_query(Matcher::Any)
            .match_header("Authorization", Matcher::Missing)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"meta": [{"name": "test", "type": "int"}], "data": [[1]]}"#)
            .create_async()
            .await;

        let mut client = FireboltClient::builder()
            .with_url(server.url())
            .build()
            .await
            .expect("core build should succeed");

        let result = client.query("SELECT 1").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_core_rejects_url_without_scheme() {
        let result = FireboltClient::builder()
            .with_url("localhost:3473".to_string())
            .build()
            .await;

        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Configuration(_)
        ));
    }

    #[tokio::test]
    async fn test_build_core_rejects_url_without_host() {
        let result = FireboltClient::builder()
            .with_url("http://".to_string())
            .build()
            .await;

        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Configuration(_)
        ));
    }

    #[tokio::test]
    async fn test_build_core_rejects_url_with_user_info() {
        let result = FireboltClient::builder()
            .with_url("http://user:pass@localhost:3473".to_string())
            .build()
            .await;

        let error = result.unwrap_err();
        assert!(matches!(error, FireboltError::Configuration(_)));
        assert!(format!("{error}").contains("credentials in the url"));
    }

    #[tokio::test]
    async fn test_build_core_rejects_url_with_query_string() {
        let result = FireboltClient::builder()
            .with_url("http://localhost:3473/core?tenant=a".to_string())
            .build()
            .await;

        let error = result.unwrap_err();
        assert!(matches!(error, FireboltError::Configuration(_)));
        assert!(format!("{error}").contains("query string"));
    }

    #[tokio::test]
    async fn test_build_core_rejects_credentials() {
        let server = mockito::Server::new_async().await;

        let result = FireboltClient::builder()
            .with_url(server.url())
            .with_credentials("id".to_string(), "secret".to_string())
            .build()
            .await;

        let error = result.unwrap_err();
        assert!(matches!(error, FireboltError::Configuration(_)));
        assert!(format!("{error}").contains("client_id"));
    }

    #[tokio::test]
    async fn test_build_core_rejects_account() {
        let server = mockito::Server::new_async().await;

        let result = FireboltClient::builder()
            .with_url(server.url())
            .with_account("my_account".to_string())
            .build()
            .await;

        let error = result.unwrap_err();
        assert!(matches!(error, FireboltError::Configuration(_)));
        assert!(format!("{error}").contains("account_name"));
    }

    #[tokio::test]
    async fn test_build_core_rejects_engine() {
        let server = mockito::Server::new_async().await;

        let result = FireboltClient::builder()
            .with_url(server.url())
            .with_engine("my_engine".to_string())
            .build()
            .await;

        let error = result.unwrap_err();
        assert!(matches!(error, FireboltError::Configuration(_)));
        assert!(format!("{error}").contains("engine"));
    }

    #[test]
    fn test_set_token_on_core_client_is_rejected() {
        let mut client = create_test_core_client("http://localhost:3473".to_string());

        let result = client.set_token("whatever".to_string());

        assert!(matches!(
            result.unwrap_err(),
            FireboltError::Configuration(_)
        ));
    }

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
        assert_eq!(client.client_id(), Some("test_id"));
        assert_eq!(client.client_secret(), Some("test_secret"));
        assert_eq!(client.api_endpoint(), Some("https://api.test.firebolt.io"));
        assert_eq!(client.engine_url(), "https://test.engine.url/");
        assert!(client.parameters().is_empty());
    }

    #[test]
    fn test_set_token() {
        let mut client = create_test_client();
        client
            .set_token("new_token".to_string())
            .expect("a cloud client accepts a token");
        assert_eq!(client._auth.as_ref().unwrap().token, "new_token");
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
            _auth: Some(CloudAuth {
                client_id: "test_id".to_string(),
                client_secret: "test_secret".to_string(),
                api_endpoint: "https://api.test.firebolt.io".to_string(),
                token: "test_token".to_string(),
            }),
            _parameters: HashMap::new(),
            _engine_url: "https://test.engine.url/".to_string(),
        }
    }

    #[tokio::test]
    async fn test_build_missing_client_id() {
        let _env = env_lock().lock().await;
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
            url: None,
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
        let _env = env_lock().lock().await;
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
            url: None,
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
        let _env = env_lock().lock().await;
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
            url: None,
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
        let _env = env_lock().lock().await;
        std::env::set_var("FIREBOLT_API_ENDPOINT", "api.test.firebolt.io");

        let factory = FireboltClientFactory {
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: Some("test_account".to_string()),
            url: None,
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
        let _env = env_lock().lock().await;
        std::env::set_var("FIREBOLT_API_ENDPOINT", "api.test.firebolt.io");

        let factory = FireboltClientFactory {
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: Some("nonexistent_account".to_string()),
            url: None,
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
        let _env = env_lock().lock().await;
        std::env::set_var("FIREBOLT_API_ENDPOINT", "api.test.firebolt.io");

        let factory = FireboltClientFactory {
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
            database_name: None,
            engine_name: None,
            account_name: Some("test_account".to_string()),
            url: None,
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
        let _env = env_lock().blocking_lock();
        std::env::remove_var("FIREBOLT_API_ENDPOINT");

        let result = FireboltClientFactory::get_api_endpoint();

        assert_eq!(result, "https://api.app.firebolt.io");
    }

    #[test]
    fn test_get_api_endpoint_from_env() {
        let _env = env_lock().blocking_lock();
        std::env::set_var("FIREBOLT_API_ENDPOINT", "custom.api.firebolt.io");

        let result = FireboltClientFactory::get_api_endpoint();

        assert_eq!(result, "https://custom.api.firebolt.io");

        std::env::remove_var("FIREBOLT_API_ENDPOINT");
    }

    #[test]
    fn test_get_api_endpoint_with_https_prefix() {
        let _env = env_lock().blocking_lock();
        std::env::set_var("FIREBOLT_API_ENDPOINT", "https://custom.api.firebolt.io");

        let result = FireboltClientFactory::get_api_endpoint();

        assert_eq!(result, "https://custom.api.firebolt.io");

        std::env::remove_var("FIREBOLT_API_ENDPOINT");
    }

    #[test]
    fn test_get_api_endpoint_with_http_prefix() {
        let _env = env_lock().blocking_lock();
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
