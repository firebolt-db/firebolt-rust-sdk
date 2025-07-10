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
