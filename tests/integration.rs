mod common;

use common::{validate_environment, TestConfig};
use firebolt::FireboltClient;

#[allow(dead_code)]
fn setup() -> Result<TestConfig, String> {
    validate_environment()?;
    TestConfig::from_env()
}

async fn create_client_from_config(config: &TestConfig) -> Result<FireboltClient, Box<dyn std::error::Error>> {
    let mut client = FireboltClient::builder()
        .with_credentials(config.client_id.clone(), config.client_secret.clone())
        .with_database(config.database.clone())
        .with_engine(config.engine.clone())
        .with_account(config.account.clone())
        .build()
        .await?;
    
    Ok(client)
}

#[tokio::test]
async fn test_use_engine_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let config = setup().map_err(|e| e)?;
    let mut client = create_client_from_config(&config).await?;
    
    let current_engine_result = client.query("SELECT CURRENT_ENGINE()").await?;
    let current_engine = current_engine_result.rows().next()
        .and_then(|row| row.get(0))
        .and_then(|val| val.as_str())
        .ok_or("Failed to get current engine")?;
    
    assert_eq!(current_engine, config.engine, "Initial engine should match config");
    
    let new_engine_name = format!("{}_new", config.engine);
    
    client.query(&format!("DROP ENGINE IF EXISTS {}", new_engine_name)).await?;
    
    client.query(&format!("CREATE ENGINE {}", new_engine_name)).await?;
    
    client.query(&format!("USE ENGINE {}", new_engine_name)).await?;
    
    let updated_engine_result = client.query("SELECT CURRENT_ENGINE()").await?;
    let updated_engine = updated_engine_result.rows().next()
        .and_then(|row| row.get(0))
        .and_then(|val| val.as_str())
        .ok_or("Failed to get updated engine")?;
    
    assert_eq!(updated_engine, new_engine_name, "Engine should be updated to new engine");
    
    client.query(&format!("USE ENGINE {}", config.engine)).await?;
    client.query(&format!("DROP ENGINE {}", new_engine_name)).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_use_database_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let config = setup().map_err(|e| e)?;
    let mut client = create_client_from_config(&config).await?;
    
    let current_database_result = client.query("SELECT CURRENT_DATABASE()").await?;
    let current_database = current_database_result.rows().next()
        .and_then(|row| row.get(0))
        .and_then(|val| val.as_str())
        .ok_or("Failed to get current database")?;
    
    assert_eq!(current_database, config.database, "Initial database should match config");
    
    let new_database_name = format!("{}_new", config.database);
    
    client.query(&format!("DROP DATABASE IF EXISTS {}", new_database_name)).await?;
    
    client.query(&format!("CREATE DATABASE {}", new_database_name)).await?;
    
    client.query(&format!("USE DATABASE {}", new_database_name)).await?;
    
    let updated_database_result = client.query("SELECT CURRENT_DATABASE()").await?;
    let updated_database = updated_database_result.rows().next()
        .and_then(|row| row.get(0))
        .and_then(|val| val.as_str())
        .ok_or("Failed to get updated database")?;
    
    assert_eq!(updated_database, new_database_name, "Database should be updated to new database");
    
    client.query(&format!("USE DATABASE {}", config.database)).await?;
    client.query(&format!("DROP DATABASE {}", new_database_name)).await?;
    
    Ok(())
}
