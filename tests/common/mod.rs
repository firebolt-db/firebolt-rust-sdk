use std::env;

#[allow(dead_code)]
pub const REQUIRED_ENV_VARS: &[&str] = &[
    "FIREBOLT_CLIENT_ID",
    "FIREBOLT_CLIENT_SECRET",
    "FIREBOLT_DATABASE",
    "FIREBOLT_ENGINE",
    "FIREBOLT_ACCOUNT",
    "FIREBOLT_API_ENDPOINT",
];

#[allow(dead_code)]
pub fn validate_environment() -> Result<(), String> {
    let mut missing_vars = Vec::new();

    for var_name in REQUIRED_ENV_VARS {
        if env::var(var_name).is_err() {
            missing_vars.push(*var_name);
        }
    }

    if !missing_vars.is_empty() {
        return Err(format!(
            "Missing required environment variables: {}. Please set these variables to run integration tests.",
            missing_vars.join(", ")
        ));
    }

    Ok(())
}

#[allow(dead_code)]
pub struct TestConfig {
    pub client_id: String,
    pub client_secret: String,
    pub database: String,
    pub engine: String,
    pub account: String,
    pub api_endpoint: String,
}

impl TestConfig {
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, String> {
        validate_environment()?;

        Ok(TestConfig {
            client_id: env::var("FIREBOLT_CLIENT_ID").unwrap(),
            client_secret: env::var("FIREBOLT_CLIENT_SECRET").unwrap(),
            database: env::var("FIREBOLT_DATABASE").unwrap(),
            engine: env::var("FIREBOLT_ENGINE").unwrap(),
            account: env::var("FIREBOLT_ACCOUNT").unwrap(),
            api_endpoint: env::var("FIREBOLT_API_ENDPOINT").unwrap(),
        })
    }
}
