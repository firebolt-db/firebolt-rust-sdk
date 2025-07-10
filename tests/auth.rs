use firebolt::authenticate;
use std::time::{SystemTime, UNIX_EPOCH};

fn get_auth_config() -> Result<(String, String, String), String> {
    let client_id = std::env::var("FIREBOLT_CLIENT_ID")
        .map_err(|_| "Missing FIREBOLT_CLIENT_ID environment variable")?;
    let client_secret = std::env::var("FIREBOLT_CLIENT_SECRET")
        .map_err(|_| "Missing FIREBOLT_CLIENT_SECRET environment variable")?;
    let api_endpoint = std::env::var("FIREBOLT_API_ENDPOINT")
        .map_err(|_| "Missing FIREBOLT_API_ENDPOINT environment variable")?;
    
    Ok((client_id, client_secret, api_endpoint))
}

#[tokio::test]
async fn test_authenticate_success() {
    let (client_id, client_secret, api_endpoint) = get_auth_config()
        .expect("Failed to load authentication configuration from environment variables");

    let result = authenticate(client_id, client_secret, api_endpoint).await;

    match result {
        Ok((access_token, expiration_timestamp)) => {
            assert!(!access_token.is_empty(), "Access token should not be empty");

            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Failed to get current time")
                .as_secs();

            assert!(
                expiration_timestamp > current_time,
                "Expiration timestamp should be in the future. Current: {current_time}, Expiration: {expiration_timestamp}"
            );

            let max_expected_expiration = current_time + 7200; // 2 hours buffer
            assert!(
                expiration_timestamp <= max_expected_expiration,
                "Expiration timestamp seems too far in the future. Expected <= {max_expected_expiration}, got {expiration_timestamp}"
            );

            println!("✅ Authentication successful");
            println!("   Token length: {} characters", access_token.len());
            println!(
                "   Expires in: {} seconds",
                expiration_timestamp - current_time
            );
        }
        Err(error) => {
            panic!("Authentication should succeed with valid credentials, but got error: {error}");
        }
    }
}

#[tokio::test]
async fn test_authenticate_invalid_credentials() {
    let (_, _, api_endpoint) = get_auth_config()
        .expect("Failed to load authentication configuration from environment variables");

    let result = authenticate(
        "invalid_client_id".to_string(),
        "invalid_client_secret".to_string(),
        api_endpoint,
    )
    .await;

    match result {
        Ok((access_token, expiration_timestamp)) => {
            panic!(
                "Authentication should fail with invalid credentials, but got success: token={access_token}, expiration={expiration_timestamp}"
            );
        }
        Err(error_message) => {
            assert!(
                !error_message.is_empty(),
                "Error message should not be empty"
            );

            let error_lower = error_message.to_lowercase();
            let contains_auth_error = error_lower.contains("invalid")
                || error_lower.contains("unauthorized")
                || error_lower.contains("client")
                || error_lower.contains("credentials")
                || error_lower.contains("authentication")
                || error_lower.contains("access_denied");

            assert!(
                contains_auth_error,
                "Error message should indicate authentication failure. Got: '{error_message}'"
            );

            println!("✅ Authentication correctly failed with invalid credentials");
            println!("   Error message: {error_message}");
        }
    }
}
