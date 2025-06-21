use reqwest::{Client as HttpClient, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Authentication failed")]
    AuthenticationFailed,
}

#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    api_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub release: String,
    pub repoid: String,
}

impl Client {
    pub fn new(endpoint: String, api_token: String, insecure: bool) -> Result<Self, ApiError> {
        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(insecure)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http: client,
            base_url: endpoint.trim_end_matches('/').to_string(),
            api_token,
        })
    }

    pub async fn get_version(&self) -> Result<VersionInfo, ApiError> {
        let url = format!("{}/api2/json/version", self.base_url);
        tracing::debug!("Fetching version from: {}", url);

        let response = self
            .http
            .get(url)
            .header("Authorization", format!("PVEAPIToken={}", self.api_token))
            .send()
            .await?;

        let status = response.status();
        tracing::debug!("Response status: {}", status);

        if status == 401 {
            return Err(ApiError::AuthenticationFailed);
        }

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("API error response: {}", error_text);
            return Err(ApiError::InvalidUrl(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        let api_response: ApiResponse<VersionInfo> = response.json().await?;
        Ok(api_response.data)
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn client_handles_successful_version_request() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api2/json/version")
            .with_header("authorization", "PVEAPIToken=user@realm!token=secret")
            .with_body(r#"{"data":{"version":"8.0.1","release":"8.0","repoid":"abc123"}}"#)
            .create_async()
            .await;

        let client =
            Client::new(server.url(), "user@realm!token=secret".to_string(), true).unwrap();

        let version = client.get_version().await.unwrap();
        assert_eq!(version.version, "8.0.1");
        assert_eq!(version.release, "8.0");
        assert_eq!(version.repoid, "abc123");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn client_handles_authentication_failure() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/api2/json/version")
            .with_status(401)
            .with_body(r#"{"errors":{"":["authentication failure"]}}"#)
            .create_async()
            .await;

        let client = Client::new(server.url(), "invalid-token".to_string(), true).unwrap();

        let result = client.get_version().await;
        match result {
            Err(ApiError::AuthenticationFailed) => {}
            _ => panic!("Expected AuthenticationFailed error"),
        }
    }

    #[tokio::test]
    async fn client_strips_trailing_slash_from_endpoint() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api2/json/version")
            .create_async()
            .await;

        let client = Client::new(format!("{}/", server.url()), "token".to_string(), true).unwrap();

        let _ = client.get_version().await;
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn client_handles_network_errors() {
        let client = Client::new(
            "http://localhost:99999".to_string(),
            "token".to_string(),
            true,
        )
        .unwrap();

        let result = client.get_version().await;
        assert!(matches!(result, Err(ApiError::Request(_))));
    }

    #[tokio::test]
    async fn client_respects_timeout() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/api2/json/version")
            .with_chunked_body(|w| {
                std::thread::sleep(std::time::Duration::from_secs(35));
                w.write_all(b"timeout")
            })
            .create_async()
            .await;

        let client = Client::new(server.url(), "token".to_string(), true).unwrap();

        let start = std::time::Instant::now();
        let result = client.get_version().await;
        let elapsed = start.elapsed();

        assert!(matches!(result, Err(ApiError::Request(_))));
        assert!(elapsed < std::time::Duration::from_secs(35));
    }
}
