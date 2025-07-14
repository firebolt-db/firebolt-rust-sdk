mod common;

use common::TestConfig;
use firebolt::{FireboltClient, FireboltError};

#[tokio::test]
async fn test_client_factory_build_integration_happy_path() {
    let config = match TestConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            println!("Skipping integration test due to setup failure: {e}");
            return;
        }
    };

    let mut factory = FireboltClient::builder()
        .with_credentials(config.client_id.clone(), config.client_secret.clone())
        .with_account(config.account.clone());

    if !config.database.is_empty() {
        factory = factory.with_database(config.database.clone());
    }

    if !config.engine.is_empty() {
        factory = factory.with_engine(config.engine.clone());
    }

    let result = factory.build().await;

    match result {
        Ok(client) => {
            assert!(!client.client_id().is_empty());
            assert!(!client.client_secret().is_empty());
            assert!(!client.engine_url().is_empty());
            assert!(!client.api_endpoint().is_empty());
        }
        Err(e) => {
            panic!("Integration test failed: {e:?}");
        }
    }
}

#[tokio::test]
async fn test_client_factory_build_integration_no_database_no_engine() {
    let config = match TestConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            println!("Skipping integration test due to setup failure: {e}");
            return;
        }
    };

    let factory = FireboltClient::builder()
        .with_credentials(config.client_id.clone(), config.client_secret.clone())
        .with_account(config.account.clone());

    let result = factory.build().await;

    match result {
        Ok(client) => {
            assert!(!client.client_id().is_empty());
            assert!(!client.client_secret().is_empty());
            assert!(!client.engine_url().is_empty());
            assert!(!client.api_endpoint().is_empty());
        }
        Err(e) => {
            panic!("Integration test failed: {e:?}");
        }
    }
}

#[tokio::test]
async fn test_client_factory_build_integration_invalid_credentials() {
    let factory = FireboltClient::builder()
        .with_credentials("invalid_id".to_string(), "invalid_secret".to_string())
        .with_account("test_account".to_string());

    let result = factory.build().await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        FireboltError::Authentication(_)
    ));
}

#[tokio::test]
async fn test_client_factory_build_integration_invalid_account() {
    let config = match TestConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            println!("Skipping integration test due to setup failure: {e}");
            return;
        }
    };

    let factory = FireboltClient::builder()
        .with_credentials(config.client_id.clone(), config.client_secret.clone())
        .with_account("nonexistent_account_12345".to_string());

    let result = factory.build().await;

    match result {
        Ok(_) => panic!("Expected error for invalid account"),
        Err(e) => {
            assert!(matches!(e, FireboltError::Configuration(_)));
        }
    }
}

#[tokio::test]
async fn test_client_factory_build_integration_missing_required_params() {
    let factory = FireboltClient::builder();

    let result = factory.build().await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        FireboltError::Configuration(_)
    ));
}
