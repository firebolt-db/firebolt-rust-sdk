# Firebolt Rust SDK

A Rust SDK for connecting to and interacting with Firebolt databases. This SDK provides an async-first interface for executing SQL queries and managing database connections with comprehensive type safety and error handling.

## Overview

The Firebolt Rust SDK enables Rust developers to connect to Firebolt databases seamlessly. It provides a builder pattern for client configuration, OAuth2 authentication, and type-safe result parsing for all Firebolt data types.

## Prerequisites

You must have the following prerequisites before you can connect your Firebolt account to Rust:

* **Rust installed and configured** on your system. The minimum supported version is 1.70 or higher. If you do not have Rust installed, you can download it from [rustup.rs](https://rustup.rs/).
* **Firebolt account** – You need an active Firebolt account. If you do not have one, you can [sign up](https://go.firebolt.io/signup) for one.
* **Firebolt service account** – You must have access to an active Firebolt [service account](https://docs.firebolt.io/guides/managing-your-organization/service-accounts), which facilitates programmatic access to Firebolt, its ID and secret.
* **Firebolt user** – You must have a user that is [associated](https://docs.firebolt.io/guides/managing-your-organization/service-accounts#create-a-user) with your service account. The user should have [USAGE](https://docs.firebolt.io/overview/security/rbac/database-permissions) permission to query your database, and [OPERATE](https://docs.firebolt.io/overview/security/rbac/engine-permissions) permission to start and stop an engine if it is not already started.
* **Firebolt database and engine (optional)** – You can optionally connect to a Firebolt database and/or engine. If you do not have one yet, you can [create a database](https://docs.firebolt.io/guides/getting-started/get-started-sql#create-a-database) and also [create an engine](https://docs.firebolt.io/guides/getting-started/get-started-sql#create-an-engine). You would need a database if you want to access stored data in Firebolt and an engine if you want to load and query stored data.

## Installation

Add the Firebolt SDK to your `Cargo.toml` dependencies:

```toml
[dependencies]
firebolt = "0.0.1"
tokio = { version = "1.0", features = ["full"] }
```

## Connect to Firebolt

To establish a connection to a Firebolt database, use the builder pattern with your credentials and database details. The following example shows how to connect to Firebolt:

```rust
use firebolt::FireboltClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = FireboltClient::builder()
        .with_credentials("your_client_id".to_string(), "your_client_secret".to_string())
        .with_account("your_account_name".to_string())
        .with_database("your_database_name".to_string())
        .with_engine("your_engine_name".to_string())
        .build()
        .await?;

    println!("Connected to Firebolt successfully!");
    Ok(())
}
```

### Connection Parameters

The SDK uses the following parameters to connect to Firebolt:

- `client_id`: Client ID of your [service account](https://docs.firebolt.io/guides/managing-your-organization/service-accounts).
- `client_secret`: Client secret of your [service account](https://docs.firebolt.io/guides/managing-your-organization/service-accounts).
- `account_name`: The name of your Firebolt [account](https://docs.firebolt.io/guides/managing-your-organization/managing-accounts).
- `database`: (Optional) The name of the [database](https://docs.firebolt.io/overview/security/rbac/database-permissions) to connect to.
- `engine`: (Optional) The name of the [engine](https://docs.firebolt.io/overview/security/rbac/engine-permissions) to run SQL queries on.

## Run Queries

Once connected, you can execute SQL queries using the `query` method. The SDK returns results with type-safe parsing for all Firebolt data types.

### Basic Query Example

```rust
use firebolt::FireboltClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = FireboltClient::builder()
        .with_credentials("your_client_id".to_string(), "your_client_secret".to_string())
        .with_account("your_account_name".to_string())
        .with_database("your_database_name".to_string())
        .with_engine("your_engine_name".to_string())
        .build()
        .await?;

    let result = client.query("SELECT 1 as test_column, 'hello' as text_column").await?;

    println!("Columns: {}", result.columns.len());
    println!("Rows: {}", result.rows.len());

    let row = &result.rows[0];
    let test_value: i32 = row.get("test_column")?;
    let text_value: String = row.get("text_column")?;

    println!("test_column: {}", test_value);
    println!("text_column: {}", text_value);

    Ok(())
}
```

### Working with Tables

```rust
use firebolt::FireboltClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = FireboltClient::builder()
        .with_credentials("your_client_id".to_string(), "your_client_secret".to_string())
        .with_account("your_account_name".to_string())
        .with_database("your_database_name".to_string())
        .with_engine("your_engine_name".to_string())
        .build()
        .await?;

    client.query("CREATE TABLE IF NOT EXISTS users (id INT, name TEXT, active BOOLEAN)").await?;

    client.query("INSERT INTO users VALUES (1, 'Alice', true), (2, 'Bob', false)").await?;

    let result = client.query("SELECT id, name, active FROM users ORDER BY id").await?;

    for row in &result.rows {
        let id: i32 = row.get("id")?;
        let name: String = row.get("name")?;
        let active: bool = row.get("active")?;

        println!("User {}: {} (active: {})", id, name, active);
    }

    Ok(())
}
```

## Type-Safe Result Parsing

The SDK provides comprehensive type conversion for all Firebolt data types. You can access column values by name or index with automatic type conversion:

### Supported Types

```rust
use firebolt::FireboltClient;
use num_bigint::BigInt;
use rust_decimal::Decimal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = FireboltClient::builder()
        .with_credentials("your_client_id".to_string(), "your_client_secret".to_string())
        .with_account("your_account_name".to_string())
        .with_database("your_database_name".to_string())
        .with_engine("your_engine_name".to_string())
        .build()
        .await?;

    let result = client.query(r#"
        SELECT
            42 as int_col,
            30000000000 as long_col,
            3.14::float4 as float_col,
            3.14159265359 as double_col,
            '123.456'::decimal(10,3) as decimal_col,
            'hello world' as text_col,
            true as bool_col,
            [1,2,3] as array_col
    "#).await?;

    let row = &result.rows[0];

    let int_val: i32 = row.get("int_col")?;
    let long_val: BigInt = row.get("long_col")?;
    let float_val: f32 = row.get("float_col")?;
    let double_val: f64 = row.get("double_col")?;
    let decimal_val: Decimal = row.get("decimal_col")?;
    let text_val: String = row.get("text_col")?;
    let bool_val: bool = row.get("bool_col")?;
    let array_val: serde_json::Value = row.get("array_col")?;

    println!("Integer: {}", int_val);
    println!("Long: {}", long_val);
    println!("Float: {}", float_val);
    println!("Double: {}", double_val);
    println!("Decimal: {}", decimal_val);
    println!("Text: {}", text_val);
    println!("Boolean: {}", bool_val);
    println!("Array: {}", array_val);

    Ok(())
}
```

### Nullable Types

For nullable columns, use `Option<T>` types:

```rust
let nullable_int: Option<i32> = row.get("nullable_column")?;
match nullable_int {
    Some(value) => println!("Value: {}", value),
    None => println!("Value is NULL"),
}
```

### Accessing by Index

You can also access columns by their index:

```rust
let first_column: String = row.get(0)?;
let second_column: i32 = row.get(1)?;
```

## Error Handling

The SDK provides comprehensive error handling through the `FireboltError` enum:

```rust
use firebolt::{FireboltClient, FireboltError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = FireboltClient::builder()
        .with_credentials("your_client_id".to_string(), "your_client_secret".to_string())
        .with_account("your_account_name".to_string())
        .build()
        .await?;

    match client.query("SELECT * FROM non_existent_table").await {
        Ok(result) => {
            println!("Query succeeded with {} rows", result.rows.len());
        }
        Err(FireboltError::Query(msg)) => {
            println!("Query error: {}", msg);
        }
        Err(FireboltError::Authentication(msg)) => {
            println!("Authentication error: {}", msg);
        }
        Err(FireboltError::Network(msg)) => {
            println!("Network error: {}", msg);
        }
        Err(FireboltError::Configuration(msg)) => {
            println!("Configuration error: {}", msg);
        }
        Err(e) => {
            println!("Other error: {}", e);
        }
    }

    Ok(())
}
```

## Environment Variables

You can configure the API endpoint using environment variables:

```bash
export FIREBOLT_API_ENDPOINT="api.staging.firebolt.io"
```

If not set, the SDK defaults to the production endpoint `api.app.firebolt.io`.

## Troubleshooting

### Common Connection Issues

| Error | Likely Cause | Solution |
|-------|--------------|----------|
| `Authentication error: Invalid credentials` | Incorrect client ID or secret | Verify your service account credentials in the Firebolt console |
| `Configuration error: client_id is required` | Missing required parameter | Ensure all required parameters are provided to the builder |
| `Network error: Failed to get engine URL` | Network connectivity issues | Check your internet connection and firewall settings |
| `Query error: Account 'account_name' not found` | Incorrect account name | Verify the account name matches exactly what's shown in the Firebolt console |

### Best Practices

- Store credentials securely using environment variables or a secrets management system
- Use connection pooling for applications with multiple concurrent queries
- Handle errors appropriately and implement retry logic for transient failures
- Use the builder pattern to configure only the parameters you need

## Additional Resources

- [Firebolt Rust SDK GitHub Repository](https://github.com/firebolt-db/firebolt-rust-sdk)
- [Firebolt Documentation](https://docs.firebolt.io/)
- [Rust Documentation](https://doc.rust-lang.org/)
