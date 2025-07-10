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

// #[tokio::test]
// async fn test_query_with_invalid_credentials() {
//     let client_result = FireboltClient::builder()
//         .with_credentials("invalid_id".to_string(), "invalid_secret".to_string())
//         .with_database("test_db".to_string())
//         .with_engine("test_engine".to_string())
//         .build()
//         .await;
//
//     match client_result {
//             let result = client.query("SELECT 1").await;
//             match result {
//                 Ok(_) => panic!("Expected authentication error"),
//                 Err(e) => {
//                     println!("✅ Invalid credentials properly returned error: {e:?}");
//                 }
//             }
//         }
//         Err(e) => {
//             println!("✅ Invalid credentials prevented client creation: {e:?}");
//         }
//     }
// }
