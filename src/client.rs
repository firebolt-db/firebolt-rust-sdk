use crate::error::FireboltError;
use crate::result::ResultSet;
use std::collections::HashMap;

pub struct FireboltClient {
    client_id: String,
    client_secret: String,
    token: Option<String>,
    parameters: HashMap<String, String>,
    engine_url: Option<String>,
    api_endpoint: String,
}

impl FireboltClient {
    pub async fn query(&self, sql: &str) -> Result<ResultSet, FireboltError> {
        todo!("FireboltClient::query implementation")
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
    api_endpoint: String,
}

impl FireboltClientFactory {
    fn new() -> Self {
        Self {
            client_id: None,
            client_secret: None,
            database_name: None,
            engine_name: None,
            account_name: None,
            api_endpoint: "https://api.firebolt.io".to_string(),
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
