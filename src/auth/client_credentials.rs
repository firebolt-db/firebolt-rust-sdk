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
    } else {
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read error response: {e}"))?;

        match serde_json::from_str::<serde_json::Value>(&response_text) {
            Ok(json) => {
                if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
                    Err(message.to_string())
                } else if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
                    Err(error.to_string())
                } else if let Some(error_description) =
                    json.get("error_description").and_then(|v| v.as_str())
                {
                    Err(error_description.to_string())
                } else {
                    Err(format!("Authentication failed: {response_text}"))
                }
            }
            Err(_) => Err(format!("Authentication failed: {response_text}")),
        }
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
}
