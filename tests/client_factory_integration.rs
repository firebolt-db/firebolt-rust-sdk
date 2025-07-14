use firebolt::{FireboltClient, FireboltError};
use std::env;

#[tokio::test]
async fn test_client_factory_build_integration_happy_path() {
    let client_id = env::var("FIREBOLT_CLIENT_ID").expect("FIREBOLT_CLIENT_ID must be set");
    let client_secret =
        env::var("FIREBOLT_CLIENT_SECRET").expect("FIREBOLT_CLIENT_SECRET must be set");
    let account = env::var("FIREBOLT_ACCOUNT").expect("FIREBOLT_ACCOUNT must be set");
    let database = env::var("FIREBOLT_DATABASE").ok();
    let engine = env::var("FIREBOLT_ENGINE").ok();

    let mut factory = FireboltClient::builder()
        .with_credentials(client_id, client_secret)
        .with_account(account);

    if let Some(db) = database {
        factory = factory.with_database(db);
    }

    if let Some(eng) = engine {
        factory = factory.with_engine(eng);
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
            if env::var("FIREBOLT_CLIENT_ID").is_err() {
                println!("Skipping integration test - environment not configured");
                return;
            }
            panic!("Integration test failed: {e:?}");
        }
    }
}

#[tokio::test]
async fn test_client_factory_build_integration_no_database_no_engine() {
    let client_id = env::var("FIREBOLT_CLIENT_ID").expect("FIREBOLT_CLIENT_ID must be set");
    let client_secret =
        env::var("FIREBOLT_CLIENT_SECRET").expect("FIREBOLT_CLIENT_SECRET must be set");
    let account = env::var("FIREBOLT_ACCOUNT").expect("FIREBOLT_ACCOUNT must be set");

    let factory = FireboltClient::builder()
        .with_credentials(client_id, client_secret)
        .with_account(account);

    let result = factory.build().await;

    match result {
        Ok(client) => {
            assert!(!client.client_id().is_empty());
            assert!(!client.client_secret().is_empty());
            assert!(!client.engine_url().is_empty());
            assert!(!client.api_endpoint().is_empty());
        }
        Err(e) => {
            if env::var("FIREBOLT_CLIENT_ID").is_err() {
                println!("Skipping integration test - environment not configured");
                return;
            }
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
    let client_id = env::var("FIREBOLT_CLIENT_ID").expect("FIREBOLT_CLIENT_ID must be set");
    let client_secret =
        env::var("FIREBOLT_CLIENT_SECRET").expect("FIREBOLT_CLIENT_SECRET must be set");

    let factory = FireboltClient::builder()
        .with_credentials(client_id, client_secret)
        .with_account("nonexistent_account_12345".to_string());

    let result = factory.build().await;

    match result {
        Ok(_) => panic!("Expected error for invalid account"),
        Err(e) => {
            if env::var("FIREBOLT_CLIENT_ID").is_err() {
                println!("Skipping integration test - environment not configured");
                return;
            }
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
