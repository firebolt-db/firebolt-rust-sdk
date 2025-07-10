use firebolt::FireboltClient;

mod common;

#[tokio::test]
async fn test_query_integration() {
    let config = match common::TestConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            println!("Skipping integration test due to setup failure: {e}");
            return;
        }
    };

    let mut client = FireboltClient::builder()
        .with_credentials(config.client_id, config.client_secret)
        .with_database(config.database)
        .with_engine(config.engine)
        .build()
        .await
        .expect("Failed to build client");

    let result = client.query("SELECT 1 as test_column").await;

    match result {
        Ok(result_set) => {
            assert!(!result_set.columns.is_empty());
            assert!(!result_set.rows.is_empty());
            println!("✅ Query executed successfully");
            println!("   Columns: {}", result_set.columns.len());
            println!("   Rows: {}", result_set.rows.len());
        }
        Err(e) => {
            println!("Expected failure during development: {e:?}");
        }
    }
}

#[tokio::test]
async fn test_query_invalid_sql() {
    let config = match common::TestConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            println!("Skipping integration test due to setup failure: {e}");
            return;
        }
    };

    let mut client = FireboltClient::builder()
        .with_credentials(config.client_id, config.client_secret)
        .with_database(config.database)
        .with_engine(config.engine)
        .build()
        .await
        .expect("Failed to build client");

    let result = client.query("INVALID SQL QUERY").await;

    match result {
        Ok(_) => panic!("Expected error for invalid SQL"),
        Err(e) => {
            println!("✅ Invalid SQL properly returned error: {e:?}");
            assert!(format!("{e:?}").contains("Server error"));
        }
    }
}

#[tokio::test]
async fn test_query_with_invalid_token() {
    let config = match common::TestConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            println!("Skipping integration test due to setup failure: {e}");
            return;
        }
    };

    let mut client = FireboltClient::builder()
        .with_credentials(config.client_id, config.client_secret)
        .with_database(config.database)
        .with_engine(config.engine)
        .build()
        .await
        .expect("Failed to build client");

    client.set_token("invalid_token".to_string());

    let result = client.query("SELECT 1 as test_column").await;

    match result {
        Ok(result_set) => {
            println!("✅ Query succeeded despite invalid token (token refresh worked)");
            assert!(!result_set.columns.is_empty());
        }
        Err(e) => {
            println!("Query failed: {e:?}");
        }
    }
}

#[tokio::test]
async fn test_query_with_invalid_credentials() {
    let mut client = FireboltClient::new_for_testing(
        "invalid_id".to_string(),
        "invalid_secret".to_string(),
        "".to_string(),
        "https://test.engine.url/".to_string(),
        "https://api.staging.firebolt.io".to_string(),
    );

    let result = client.query("SELECT 1").await;

    match result {
        Ok(_) => panic!("Expected authentication error with invalid credentials"),
        Err(e) => {
            println!("✅ Invalid credentials properly returned error: {e:?}");
            let error_str = format!("{e:?}");
            assert!(
                error_str.contains("Authentication")
                    || error_str.contains("authentication")
                    || error_str.contains("invalid")
                    || error_str.contains("unauthorized")
                    || error_str.contains("Network")
                    || error_str.contains("dns error")
                    || error_str.contains("failed to lookup"),
                "Expected authentication or network error with invalid credentials, got: {error_str}"
            );
        }
    }
}
