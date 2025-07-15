mod common;

use common::{validate_environment, TestConfig};
use firebolt::FireboltClient;

#[allow(dead_code)]
fn setup() -> Result<TestConfig, String> {
    validate_environment()?;
    TestConfig::from_env()
}

async fn create_client_from_config(
    config: &TestConfig,
) -> Result<FireboltClient, Box<dyn std::error::Error>> {
    let client = FireboltClient::builder()
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
    let config = setup()?;
    let mut client = create_client_from_config(&config).await?;

    let current_engine_result = client.query("SELECT CURRENT_ENGINE()").await?;
    let current_engine = current_engine_result
        .rows
        .first()
        .and_then(|row| row.get::<String>(0).ok())
        .ok_or("Failed to get current engine")?;

    assert_eq!(
        current_engine, config.engine,
        "Initial engine should match config"
    );

    let new_engine_name = format!("{}_new", config.engine);

    client
        .query(&format!("CREATE ENGINE IF NOT EXISTS {new_engine_name}"))
        .await?;

    client
        .query(&format!("USE ENGINE {new_engine_name}"))
        .await?;

    let updated_engine_result = client.query("SELECT CURRENT_ENGINE()").await?;
    let updated_engine = updated_engine_result
        .rows
        .first()
        .and_then(|row| row.get::<String>(0).ok())
        .ok_or("Failed to get updated engine")?;

    assert_eq!(
        updated_engine, new_engine_name,
        "Engine should be updated to new engine"
    );

    client
        .query(&format!("USE ENGINE {}", config.engine))
        .await?;
    client
        .query(&format!("STOP ENGINE {new_engine_name}"))
        .await?;
    client
        .query(&format!("DROP ENGINE {new_engine_name}"))
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_use_database_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let config = setup()?;
    let mut client = create_client_from_config(&config).await?;

    let current_database_result = client.query("SELECT CURRENT_DATABASE()").await?;
    let current_database = current_database_result
        .rows
        .first()
        .and_then(|row| row.get::<String>(0).ok())
        .ok_or("Failed to get current database")?;

    assert_eq!(
        current_database, config.database,
        "Initial database should match config"
    );

    let new_database_name = format!("{}_new", config.database);

    client
        .query(&format!("DROP DATABASE IF EXISTS {new_database_name}"))
        .await?;

    client
        .query(&format!("CREATE DATABASE {new_database_name}"))
        .await?;

    client
        .query(&format!("USE DATABASE {new_database_name}"))
        .await?;

    let updated_database_result = client.query("SELECT CURRENT_DATABASE()").await?;
    let updated_database = updated_database_result
        .rows
        .first()
        .and_then(|row| row.get::<String>(0).ok())
        .ok_or("Failed to get updated database")?;

    assert_eq!(
        updated_database, new_database_name,
        "Database should be updated to new database"
    );

    client
        .query(&format!("USE DATABASE {}", config.database))
        .await?;
    client
        .query(&format!("DROP DATABASE {new_database_name}"))
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_all_data_types_parsing() -> Result<(), Box<dyn std::error::Error>> {
    let config = setup()?;
    let mut client = create_client_from_config(&config).await?;

    let query = r#"
        select  1                                                         as col_int,
                null::int                                                 as col_int_null,
                30000000000                                               as col_long,
                null::bigint                                              as col_long_null,
                1.23::float4                                              as col_float,
                null::float4                                              as col_float_null,
                1.23456789012                                             as col_double,
                null::double                                              as col_double_null,
                'text'                                                    as col_text,
                null::text                                                as col_text_null,
                '2021-03-28'::date                                        as col_date,
                null::date                                                as col_date_null,
                '2019-07-31 01:01:01'::timestamp                          as col_timestamp,
                null::timestamp                                           as col_timestamp_null,
                '1111-01-05 17:04:42.123456'::timestamptz                 as col_timestamptz,
                null::timestamptz                                         as col_timestamptz_null,
                true                                                      as col_boolean,
                null::bool                                                as col_boolean_null,
                [1,2,3,4]                                                 as col_array,
                null::array(int)                                          as col_array_null,
                '1231232.123459999990457054844258706536'::decimal(38, 30) as col_decimal,
                null::decimal(38, 30)                                     as col_decimal_null,
                'abc123'::bytea                                           as col_bytea,
                null::bytea                                               as col_bytea_null,
                'point(1 2)'::geography                                   as col_geography,
                null::geography                                           as col_geography_null
    "#;

    let result = client.query(query).await?;

    assert_eq!(result.columns.len(), 26);
    assert_eq!(result.rows.len(), 1);

    let row = &result.rows[0];

    let col_int: i32 = row.get("col_int")?;
    assert_eq!(col_int, 1);

    let col_int_null: Option<i32> = row.get("col_int_null")?;
    assert_eq!(col_int_null, None);

    let col_long: num_bigint::BigInt = row.get("col_long")?;
    assert_eq!(col_long, num_bigint::BigInt::from(30000000000i64));

    let col_long_null: Option<num_bigint::BigInt> = row.get("col_long_null")?;
    assert_eq!(col_long_null, None);

    let col_float: f32 = row.get("col_float")?;
    assert!((col_float - 1.23).abs() < 0.01);

    let col_float_null: Option<f32> = row.get("col_float_null")?;
    assert_eq!(col_float_null, None);

    let col_double: f64 = row.get("col_double")?;
    assert!((col_double - 1.23456789012).abs() < 0.0001);

    let col_double_null: Option<f64> = row.get("col_double_null")?;
    assert_eq!(col_double_null, None);

    let col_text: String = row.get("col_text")?;
    assert_eq!(col_text, "text");

    let col_text_null: Option<String> = row.get("col_text_null")?;
    assert_eq!(col_text_null, None);

    let col_boolean: bool = row.get("col_boolean")?;
    assert!(col_boolean);

    let col_boolean_null: Option<bool> = row.get("col_boolean_null")?;
    assert_eq!(col_boolean_null, None);

    let col_decimal: rust_decimal::Decimal = row.get("col_decimal")?;
    let expected_decimal: rust_decimal::Decimal =
        "1231232.123459999990457054844258706536".parse()?;
    assert_eq!(col_decimal, expected_decimal);

    let col_decimal_null: Option<rust_decimal::Decimal> = row.get("col_decimal_null")?;
    assert_eq!(col_decimal_null, None);

    let col_array: serde_json::Value = row.get("col_array")?;
    assert!(col_array.is_array());

    let col_array_null: serde_json::Value = row.get("col_array_null")?;
    assert!(col_array_null.is_null());

    let col_bytea: Vec<u8> = row.get("col_bytea")?;
    assert!(!col_bytea.is_empty());

    let col_bytea_null: Option<Vec<u8>> = row.get("col_bytea_null")?;
    assert_eq!(col_bytea_null, None);

    let col_geography: serde_json::Value = row.get("col_geography")?;
    assert!(col_geography.is_string());

    let col_geography_null: serde_json::Value = row.get("col_geography_null")?;
    assert!(col_geography_null.is_null());

    Ok(())
}
