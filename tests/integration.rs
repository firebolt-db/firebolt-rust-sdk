

use firebolt::client::Client;

mod common;

use common::{validate_environment, TestConfig};

fn setup() -> Result<TestConfig, String> {
    validate_environment()?;
    TestConfig::from_env()
}

#[test]
fn test_environment_validation() {
    match validate_environment() {
        Ok(()) => {
            println!("All required environment variables are set");
        }
        Err(msg) => {
            println!("Environment validation failed: {}", msg);
            println!("Skipping integration tests due to missing environment variables");
            return;
        }
    }
}

#[test]
fn test_client_creation_with_config() {
    let _config = match setup() {
        Ok(config) => config,
        Err(msg) => {
            println!("Skipping test due to environment setup failure: {}", msg);
            return;
        }
    };
    
    let _client = Client::new();
}
