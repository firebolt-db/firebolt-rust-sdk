# Firebolt Rust SDK - Comprehensive Documentation

This document provides comprehensive documentation for the Firebolt Rust SDK, covering advanced usage patterns, API reference, and detailed examples.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Configuration](#configuration)
3. [Authentication](#authentication)
4. [Client API Reference](#client-api-reference)
5. [Data Types and Type Conversion](#data-types-and-type-conversion)
6. [Error Handling](#error-handling)
7. [Advanced Features](#advanced-features)
8. [Performance Considerations](#performance-considerations)
9. [Examples](#examples)
10. [Troubleshooting](#troubleshooting)

## Quick Start

For basic usage, see the [README.md](README.md). This document covers advanced usage patterns and detailed API reference.

## Configuration

### Environment Variables

The SDK supports several environment variables for configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `FIREBOLT_API_ENDPOINT` | Firebolt API endpoint URL | `api.app.firebolt.io` |

### Builder Pattern Configuration

The `FireboltClientFactory` provides a fluent builder interface:

```rust
use firebolt::FireboltClient;

let client = FireboltClient::builder()
    .with_credentials(client_id, client_secret)  // Required
    .with_account(account_name)                  // Required
    .with_database(database_name)               // Optional
    .with_engine(engine_name)                   // Optional
    .build()
    .await?;
```

## Authentication

### OAuth2 Client Credentials Flow

The SDK uses OAuth2 client credentials flow for authentication:

1. **Endpoint Transformation**: API endpoint `api.<env>.firebolt.io` is transformed to `id.<env>.firebolt.io/oauth/token`
2. **Token Request**: POST request with `client_id`, `client_secret`, `grant_type: "client_credentials"`, and `audience: "https://api.firebolt.io"`
3. **Token Management**: Automatic token refresh on 401 responses
4. **Expiration Tracking**: Local calculation of token expiration timestamp

### Authentication Error Handling

```rust
use firebolt::{FireboltClient, FireboltError};

match FireboltClient::builder()
    .with_credentials("invalid_id".to_string(), "invalid_secret".to_string())
    .with_account("test_account".to_string())
    .build()
    .await
{
    Ok(client) => println!("Authentication successful"),
    Err(FireboltError::Authentication(msg)) => {
        eprintln!("Authentication failed: {}", msg);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Client API Reference

### FireboltClient

#### Methods

##### `query(&mut self, sql: &str) -> Result<ResultSet, FireboltError>`

Executes a SQL query and returns the result set.

**Parameters:**
- `sql`: SQL query string

**Returns:**
- `Ok(ResultSet)`: Query results with columns and rows
- `Err(FireboltError)`: Various error types (see Error Handling section)

**Features:**
- Automatic token refresh on authentication failure
- Dynamic parameter and endpoint updates from server headers
- Comprehensive error handling

##### Getter Methods

- `client_id(&self) -> &str`: Returns the client ID
- `client_secret(&self) -> &str`: Returns the client secret
- `api_endpoint(&self) -> &str`: Returns the API endpoint URL
- `engine_url(&self) -> &str`: Returns the current engine URL
- `parameters(&self) -> &HashMap<String, String>`: Returns current session parameters

### FireboltClientFactory

#### Builder Methods

##### `with_credentials(self, client_id: String, client_secret: String) -> Self`

Sets authentication credentials.

##### `with_account(self, account_name: String) -> Self`

Sets the Firebolt account name (required).

##### `with_database(self, database_name: String) -> Self`

Sets the default database. Executes `USE DATABASE` during client initialization.

##### `with_engine(self, engine_name: String) -> Self`

Sets the default engine. Executes `USE ENGINE` during client initialization.

##### `build(self) -> Result<FireboltClient, FireboltError>`

Builds the client with the configured parameters.

**Process:**
1. Validates required parameters
2. Authenticates with Firebolt
3. Retrieves engine URL
4. Optionally sets database and engine
5. Returns configured client

## Data Types and Type Conversion

### Supported Firebolt Types

| Firebolt Type | Rust Type | Nullable Rust Type | Notes |
|---------------|-----------|-------------------|-------|
| `int` | `i32` | `Option<i32>` | 32-bit signed integer |
| `bigint`, `long` | `num_bigint::BigInt` | `Option<num_bigint::BigInt>` | Arbitrary precision integer |
| `float4`, `float` | `f32` | `Option<f32>` | 32-bit floating point |
| `double`, `float8` | `f64` | `Option<f64>` | 64-bit floating point |
| `decimal(p,s)` | `rust_decimal::Decimal` | `Option<rust_decimal::Decimal>` | Fixed-point decimal |
| `text`, `string` | `String` | `Option<String>` | UTF-8 text |
| `date` | `serde_json::Value` | `serde_json::Value` | Date values |
| `timestamp` | `serde_json::Value` | `serde_json::Value` | Timestamp values |
| `timestamptz` | `serde_json::Value` | `serde_json::Value` | Timezone-aware timestamp |
| `bool`, `boolean` | `bool` | `Option<bool>` | Boolean values |
| `array(T)` | `serde_json::Value` | `serde_json::Value` | Array of any type |
| `bytea` | `Vec<u8>` | `Option<Vec<u8>>` | Binary data |
| `geography` | `serde_json::Value` | `serde_json::Value` | Geographic data |

### Type Conversion Examples

```rust
use firebolt::FireboltClient;
use num_bigint::BigInt;
use rust_decimal::Decimal;

let mut client = /* ... initialize client ... */;

let result = client.query(r#"
    SELECT 
        42 as int_col,
        30000000000 as bigint_col,
        '123.456'::decimal(10,3) as decimal_col,
        'hello' as text_col,
        true as bool_col,
        [1,2,3] as array_col,
        NULL as nullable_col
"#).await?;

let row = &result.rows[0];

// Type-safe access by column name
let int_val: i32 = row.get("int_col")?;
let bigint_val: BigInt = row.get("bigint_col")?;
let decimal_val: Decimal = row.get("decimal_col")?;
let text_val: String = row.get("text_col")?;
let bool_val: bool = row.get("bool_col")?;
let array_val: serde_json::Value = row.get("array_col")?;

// Nullable types
let nullable_val: Option<i32> = row.get("nullable_col")?;

// Access by column index
let first_col: i32 = row.get(0)?;
```

### Custom Type Conversion

The SDK uses the `TypeConversion` trait for type conversion. You can access raw JSON values:

```rust
let raw_value: serde_json::Value = row.get("any_column")?;
```

## Error Handling

### Error Types

The `FireboltError` enum provides structured error handling:

```rust
#[derive(Error, Debug)]
pub enum FireboltError {
    Authentication(String),    // OAuth2 authentication failures
    Network(String),          // Network connectivity issues
    Query(String),            // SQL query errors
    Serialization(String),    // JSON parsing/serialization errors
    Configuration(String),    // Client configuration errors
    HeaderParsing(String),    // Server header parsing errors
    Unknown(String),          // Unexpected errors
}
```

### Error Handling Patterns

#### Comprehensive Error Handling

```rust
use firebolt::{FireboltClient, FireboltError};

match client.query("SELECT * FROM table").await {
    Ok(result) => {
        println!("Query succeeded with {} rows", result.rows.len());
    }
    Err(FireboltError::Authentication(msg)) => {
        eprintln!("Authentication error: {}", msg);
        // Handle re-authentication
    }
    Err(FireboltError::Network(msg)) => {
        eprintln!("Network error: {}", msg);
        // Handle retry logic
    }
    Err(FireboltError::Query(msg)) => {
        eprintln!("SQL error: {}", msg);
        // Handle SQL syntax or semantic errors
    }
    Err(FireboltError::Configuration(msg)) => {
        eprintln!("Configuration error: {}", msg);
        // Handle client setup issues
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

#### Type Conversion Error Handling

```rust
let row = &result.rows[0];

match row.get::<i32>("column_name") {
    Ok(value) => println!("Value: {}", value),
    Err(FireboltError::Serialization(msg)) => {
        eprintln!("Type conversion failed: {}", msg);
        // Handle type mismatch
    }
    Err(FireboltError::Query(msg)) => {
        eprintln!("Column access failed: {}", msg);
        // Handle missing column
    }
    Err(e) => eprintln!("Unexpected error: {}", e),
}
```

## Advanced Features

### Dynamic Parameter Updates

The client automatically handles server-sent parameter updates via HTTP headers:

- `Firebolt-Update-Endpoint`: Updates the engine URL
- `Firebolt-Update-Parameters`: Updates session parameters
- `Firebolt-Reset-Session`: Resets session parameters (preserving database/engine)
- `Firebolt-Remove-Parameters`: Removes specific parameters

These updates happen automatically during query execution.

### Session Management

```rust
// Check current parameters
let params = client.parameters();
println!("Current parameters: {:?}", params);

// Parameters are updated automatically by the server
client.query("SET some_parameter = 'value'").await?;

// The client now has updated parameters
let updated_params = client.parameters();
```

### Engine URL Management

```rust
// Initial engine URL from account configuration
println!("Engine URL: {}", client.engine_url());

// URL may be updated by server responses
client.query("USE ENGINE different_engine").await?;

// Engine URL is automatically updated
println!("Updated Engine URL: {}", client.engine_url());
```

## Performance Considerations

### Connection Reuse

- The client maintains a persistent connection and reuses it for multiple queries
- Authentication tokens are cached and automatically refreshed
- HTTP/2 connection pooling is handled by the underlying `reqwest` client

### Async/Await Best Practices

```rust
use tokio::time::{timeout, Duration};

// Set timeouts for long-running queries
let result = timeout(
    Duration::from_secs(300), // 5 minute timeout
    client.query("SELECT * FROM large_table")
).await??;

// Concurrent queries (if using multiple clients)
let (result1, result2) = tokio::join!(
    client1.query("SELECT COUNT(*) FROM table1"),
    client2.query("SELECT COUNT(*) FROM table2")
);
```

### Memory Management

- Large result sets are loaded into memory
- Consider pagination for very large datasets:

```rust
let page_size = 10000;
let mut offset = 0;

loop {
    let result = client.query(&format!(
        "SELECT * FROM large_table LIMIT {} OFFSET {}",
        page_size, offset
    )).await?;
    
    if result.rows.is_empty() {
        break;
    }
    
    // Process batch
    for row in &result.rows {
        // Process row
    }
    
    offset += page_size;
}
```

## Examples

### Complete Application Example

```rust
use firebolt::{FireboltClient, FireboltError};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let mut client = FireboltClient::builder()
        .with_credentials(
            std::env::var("FIREBOLT_CLIENT_ID")?,
            std::env::var("FIREBOLT_CLIENT_SECRET")?
        )
        .with_account(std::env::var("FIREBOLT_ACCOUNT")?)
        .with_database(std::env::var("FIREBOLT_DATABASE")?)
        .with_engine(std::env::var("FIREBOLT_ENGINE")?)
        .build()
        .await?;

    // Create table
    client.query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id INT,
            name TEXT,
            email TEXT,
            created_at TIMESTAMP
        )
    "#).await?;

    // Insert data
    client.query(r#"
        INSERT INTO users VALUES 
        (1, 'Alice', 'alice@example.com', '2023-01-01 10:00:00'),
        (2, 'Bob', 'bob@example.com', '2023-01-02 11:00:00')
    "#).await?;

    // Query data
    let result = client.query("SELECT * FROM users ORDER BY id").await?;

    println!("Found {} users:", result.rows.len());
    for row in &result.rows {
        let id: i32 = row.get("id")?;
        let name: String = row.get("name")?;
        let email: String = row.get("email")?;
        
        println!("User {}: {} ({})", id, name, email);
    }

    Ok(())
}
```

### Data Type Showcase

```rust
use firebolt::FireboltClient;
use num_bigint::BigInt;
use rust_decimal::Decimal;

async fn showcase_data_types(client: &mut FireboltClient) -> Result<(), Box<dyn std::error::Error>> {
    let result = client.query(r#"
        SELECT 
            42 as int_value,
            30000000000 as bigint_value,
            3.14159::float4 as float_value,
            3.141592653589793 as double_value,
            '123.456789'::decimal(10,6) as decimal_value,
            'Hello, Firebolt!' as text_value,
            true as boolean_value,
            [1, 2, 3, 4, 5] as array_value,
            '2023-12-25'::date as date_value,
            '2023-12-25 15:30:00'::timestamp as timestamp_value,
            'abc123'::bytea as bytea_value,
            NULL as null_value
    "#).await?;

    let row = &result.rows[0];

    // Extract and display each type
    let int_val: i32 = row.get("int_value")?;
    println!("Integer: {}", int_val);

    let bigint_val: BigInt = row.get("bigint_value")?;
    println!("BigInt: {}", bigint_val);

    let float_val: f32 = row.get("float_value")?;
    println!("Float: {}", float_val);

    let double_val: f64 = row.get("double_value")?;
    println!("Double: {}", double_val);

    let decimal_val: Decimal = row.get("decimal_value")?;
    println!("Decimal: {}", decimal_val);

    let text_val: String = row.get("text_value")?;
    println!("Text: {}", text_val);

    let bool_val: bool = row.get("boolean_value")?;
    println!("Boolean: {}", bool_val);

    let array_val: serde_json::Value = row.get("array_value")?;
    println!("Array: {}", array_val);

    let bytea_val: Vec<u8> = row.get("bytea_value")?;
    println!("Bytea length: {}", bytea_val.len());

    // Handle nullable value
    let null_val: Option<i32> = row.get("null_value")?;
    match null_val {
        Some(val) => println!("Null value: {}", val),
        None => println!("Null value: NULL"),
    }

    Ok(())
}
```

### Error Recovery Example

```rust
use firebolt::{FireboltClient, FireboltError};
use tokio::time::{sleep, Duration};

async fn robust_query_with_retry(
    client: &mut FireboltClient,
    sql: &str,
    max_retries: u32
) -> Result<firebolt::ResultSet, FireboltError> {
    let mut attempts = 0;
    
    loop {
        match client.query(sql).await {
            Ok(result) => return Ok(result),
            Err(FireboltError::Network(msg)) if attempts < max_retries => {
                attempts += 1;
                eprintln!("Network error (attempt {}): {}. Retrying...", attempts, msg);
                sleep(Duration::from_secs(2_u64.pow(attempts))).await; // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Troubleshooting

### Common Issues and Solutions

#### Authentication Failures

**Problem**: `Authentication error: Invalid credentials`

**Solutions**:
1. Verify client ID and secret in Firebolt console
2. Check service account permissions
3. Ensure account name is correct
4. Verify API endpoint format

#### Network Connectivity

**Problem**: `Network error: Failed to get engine URL`

**Solutions**:
1. Check internet connectivity
2. Verify firewall settings allow HTTPS traffic
3. Check if API endpoint is accessible
4. Verify account name exists

#### Type Conversion Errors

**Problem**: `Serialization error: Cannot convert Long to i32`

**Solutions**:
1. Use appropriate Rust type (`BigInt` for `bigint`)
2. Check column data types in your query
3. Use `Option<T>` for nullable columns
4. Use `serde_json::Value` for unknown types

#### Query Errors

**Problem**: `Query error: relation "table_name" does not exist`

**Solutions**:
1. Verify table name spelling
2. Check if you're connected to the correct database
3. Ensure table exists: `SHOW TABLES`
4. Check table permissions

### Debug Information

Enable debug logging to troubleshoot issues:

```rust
// Set environment variable
std::env::set_var("RUST_LOG", "debug");

// Initialize logger (using env_logger)
env_logger::init();

// SDK will now output debug information
```

### Performance Debugging

Monitor query performance:

```rust
use std::time::Instant;

let start = Instant::now();
let result = client.query("SELECT COUNT(*) FROM large_table").await?;
let duration = start.elapsed();

println!("Query took: {:?}", duration);
println!("Rows returned: {}", result.rows.len());
```

### Connection State Inspection

```rust
// Check client state
println!("API Endpoint: {}", client.api_endpoint());
println!("Engine URL: {}", client.engine_url());
println!("Parameters: {:?}", client.parameters());

// Test connectivity
match client.query("SELECT 1").await {
    Ok(_) => println!("Connection is healthy"),
    Err(e) => eprintln!("Connection issue: {}", e),
}
```

## Additional Resources

- [Firebolt Documentation](https://docs.firebolt.io/)
- [Rust Documentation](https://doc.rust-lang.org/)
- [SDK Source Code](https://github.com/firebolt-db/firebolt-rust-sdk)
- [Issue Tracker](https://github.com/firebolt-db/firebolt-rust-sdk/issues)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on contributing to the SDK.

## License

This SDK is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.
