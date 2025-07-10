use crate::version::user_agent;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct AuthRequest {
    client_id: String,
    client_secret: String,
    grant_type: String,
    audience: String,
}

#[derive(Deserialize)]
struct AuthResponse {
    access_token: String,
    expires_in: u64,
}

pub async fn authenticate(
    client_id: String,
    client_secret: String,
    api_endpoint: String,
) -> Result<(String, u64), String> {
    let auth_url = validate_and_transform_endpoint(&api_endpoint)?;

    let auth_request = AuthRequest {
        client_id,
        client_secret,
        grant_type: "client_credentials".to_string(),
        audience: "https://api.firebolt.io".to_string(),
    };

    let client = Client::new();

    let response = client
        .post(&auth_url)
        .header("User-Agent", user_agent())
        .json(&auth_request)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = response.status();

    if status.is_success() {
        handle_success_response(response).await
    } else {
        handle_error_response(response).await
    }
}

async fn handle_success_response(response: reqwest::Response) -> Result<(String, u64), String> {
    let auth_response: AuthResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to get current time: {e}"))?
        .as_secs();

    let expiration_timestamp = current_time + auth_response.expires_in;

    Ok((auth_response.access_token, expiration_timestamp))
}

async fn handle_error_response(response: reqwest::Response) -> Result<(String, u64), String> {
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read error response: {e}"))?;

    Err(extract_error_message_from_json(&response_text))
}

fn extract_error_message_from_json(response_text: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(response_text) {
        Ok(json) => {
            if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
                message.to_string()
            } else if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
                error.to_string()
            } else if let Some(error_description) =
                json.get("error_description").and_then(|v| v.as_str())
            {
                error_description.to_string()
            } else {
                format!("Authentication failed: {response_text}")
            }
        }
        Err(_) => format!("Authentication failed: {response_text}"),
    }
}

fn validate_and_transform_endpoint(api_endpoint: &str) -> Result<String, String> {
    let endpoint = api_endpoint
        .strip_prefix("https://")
        .or_else(|| api_endpoint.strip_prefix("http://"))
        .unwrap_or(api_endpoint);

    if !endpoint.starts_with("api.") || !endpoint.ends_with(".firebolt.io") {
        return Err(format!(
            "Invalid API endpoint format. Expected 'api.<env>.firebolt.io', got '{endpoint}'"
        ));
    }

    let auth_endpoint = endpoint.replacen("api.", "id.", 1);

    Ok(format!("https://{auth_endpoint}/oauth/token"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_validate_and_transform_endpoint() {
        assert_eq!(
            validate_and_transform_endpoint("api.dev.firebolt.io").unwrap(),
            "https://id.dev.firebolt.io/oauth/token"
        );

        assert_eq!(
            validate_and_transform_endpoint("https://api.staging.firebolt.io").unwrap(),
            "https://id.staging.firebolt.io/oauth/token"
        );

        assert_eq!(
            validate_and_transform_endpoint("api.firebolt.io").unwrap(),
            "https://id.firebolt.io/oauth/token"
        );

        assert!(validate_and_transform_endpoint("invalid.endpoint.com").is_err());
        assert!(validate_and_transform_endpoint("api.invalid.com").is_err());
        assert!(validate_and_transform_endpoint("wrong.dev.firebolt.io").is_err());
    }

    #[test]
    fn test_auth_request_serialization() {
        let auth_request = AuthRequest {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            grant_type: "client_credentials".to_string(),
            audience: "https://api.firebolt.io".to_string(),
        };

        let json = serde_json::to_string(&auth_request).unwrap();
        assert!(json.contains("\"client_id\":\"test_client\""));
        assert!(json.contains("\"client_secret\":\"test_secret\""));
        assert!(json.contains("\"grant_type\":\"client_credentials\""));
        assert!(json.contains("\"audience\":\"https://api.firebolt.io\""));
    }

    #[test]
    fn test_auth_response_deserialization() {
        let json = r#"{"access_token": "test_token_123", "expires_in": 3600}"#;
        let auth_response: AuthResponse = serde_json::from_str(json).unwrap();

        assert_eq!(auth_response.access_token, "test_token_123");
        assert_eq!(auth_response.expires_in, 3600);
    }

    #[test]
    fn test_auth_response_deserialization_invalid() {
        let json = r#"{"invalid": "data"}"#;
        let result: Result<AuthResponse, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_success_response() {
        let json_response = r#"{"access_token": "test_token_123", "expires_in": 3600}"#;

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_response)
            .create_async()
            .await;

        let client = Client::new();
        let response = client
            .post(format!("{}/test", server.url()))
            .send()
            .await
            .unwrap();

        let result = handle_success_response(response).await;

        mock.assert_async().await;

        match result {
            Ok((access_token, expiration_timestamp)) => {
                assert_eq!(access_token, "test_token_123");
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                assert!(expiration_timestamp > current_time);
                assert!(expiration_timestamp <= current_time + 3600);
            }
            Err(error) => panic!("Expected success, got error: {error}"),
        }
    }

    #[tokio::test]
    async fn test_handle_error_response_with_message() {
        let json_response = r#"{"message": "Invalid credentials"}"#;

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(json_response)
            .create_async()
            .await;

        let client = Client::new();
        let response = client
            .post(format!("{}/test", server.url()))
            .send()
            .await
            .unwrap();

        let result = handle_error_response(response).await;

        mock.assert_async().await;

        match result {
            Ok(_) => panic!("Expected error, got success"),
            Err(error_message) => {
                assert_eq!(error_message, "Invalid credentials");
            }
        }
    }

    #[tokio::test]
    async fn test_handle_error_response_with_error_field() {
        let json_response = r#"{"error": "invalid_client"}"#;

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(json_response)
            .create_async()
            .await;

        let client = Client::new();
        let response = client
            .post(format!("{}/test", server.url()))
            .send()
            .await
            .unwrap();

        let result = handle_error_response(response).await;

        mock.assert_async().await;

        match result {
            Ok(_) => panic!("Expected error, got success"),
            Err(error_message) => {
                assert_eq!(error_message, "invalid_client");
            }
        }
    }
}
