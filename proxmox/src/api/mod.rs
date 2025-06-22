use reqwest::{Client as HttpClient, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    pub async fn get_realm(&self, realm: &str) -> Result<Option<RealmInfo>, ApiError> {
        let url = format!("{}/api2/json/access/domains/{}", self.base_url, realm);
        tracing::debug!("Fetching realm from: {}", url);

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

        // Proxmox returns 500 when realm doesn't exist
        if status == 500 {
            let error_text = response.text().await.unwrap_or_default();
            if error_text.contains("does not exist") {
                return Ok(None);
            }
            return Err(ApiError::InvalidUrl(format!(
                "API returned 500: {}",
                error_text
            )));
        }

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiError::InvalidUrl(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        let api_response: ApiResponse<RealmInfo> = response.json().await?;
        Ok(Some(api_response.data))
    }

    pub async fn create_realm(&self, config: RealmConfig) -> Result<(), ApiError> {
        let url = format!("{}/api2/json/access/domains", self.base_url);
        tracing::debug!("Creating realm at: {}", url);
        tracing::debug!("Realm config: {:?}", config);

        let mut form_data = HashMap::new();
        form_data.insert("realm", config.realm);
        form_data.insert("type", config.realm_type);
        form_data.insert("issuer-url", config.issuer_url);
        form_data.insert("client-id", config.client_id);
        form_data.insert("client-key", config.client_key);

        if let Some(username_claim) = config.username_claim {
            form_data.insert("username-claim", username_claim);
        }

        if let Some(autocreate) = config.autocreate {
            form_data.insert(
                "autocreate",
                if autocreate {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            );
        }

        if let Some(default) = config.default {
            form_data.insert(
                "default",
                if default {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            );
        }

        if let Some(groups_overwrite) = config.groups_overwrite {
            form_data.insert(
                "groups-overwrite",
                if groups_overwrite {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            );
        }

        if let Some(groups_autocreate) = config.groups_autocreate {
            form_data.insert(
                "groups-autocreate",
                if groups_autocreate {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            );
        }

        if let Some(comment) = config.comment {
            form_data.insert("comment", comment);
        }

        tracing::debug!("Sending POST request with form data: {:?}", form_data);

        let auth_header = format!("PVEAPIToken={}", self.api_token);
        tracing::debug!("Authorization header: {}", auth_header);

        let response = self
            .http
            .post(&url)
            .header("Authorization", auth_header)
            .form(&form_data)
            .send()
            .await?;

        let status = response.status();
        tracing::debug!("Response status: {}", status);

        if status == 401 {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("Authentication failed. Server response: {}", error_text);
            return Err(ApiError::AuthenticationFailed);
        }

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if status == 400 && error_text.contains("already exists") {
                return Err(ApiError::InvalidUrl(format!(
                    "Realm already exists: {}",
                    error_text
                )));
            }

            return Err(ApiError::InvalidUrl(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        Ok(())
    }

    pub async fn update_realm(&self, realm: &str, config: RealmConfig) -> Result<(), ApiError> {
        let url = format!("{}/api2/json/access/domains/{}", self.base_url, realm);
        tracing::debug!("Updating realm at: {}", url);

        let mut form_data = HashMap::new();

        // For OIDC realms, only certain fields can be updated
        // Always include required fields
        form_data.insert("issuer-url".to_string(), config.issuer_url);
        form_data.insert("client-id".to_string(), config.client_id);
        form_data.insert("client-key".to_string(), config.client_key);

        // Only include optional fields that can be updated
        if let Some(autocreate) = config.autocreate {
            form_data.insert(
                "autocreate".to_string(),
                if autocreate { "1" } else { "0" }.to_string(),
            );
        }

        if let Some(default) = config.default {
            form_data.insert(
                "default".to_string(),
                if default { "1" } else { "0" }.to_string(),
            );
        }

        if let Some(comment) = config.comment {
            form_data.insert("comment".to_string(), comment);
        }

        if let Some(groups_overwrite) = config.groups_overwrite {
            form_data.insert(
                "groups-overwrite".to_string(),
                if groups_overwrite { "1" } else { "0" }.to_string(),
            );
        }

        if let Some(groups_autocreate) = config.groups_autocreate {
            form_data.insert(
                "groups-autocreate".to_string(),
                if groups_autocreate { "1" } else { "0" }.to_string(),
            );
        }

        // Note: username-claim is not included in updates as Proxmox API doesn't accept it

        tracing::debug!("Form data for update: {:?}", form_data);

        let response = self
            .http
            .put(url)
            .header("Authorization", format!("PVEAPIToken={}", self.api_token))
            .form(&form_data)
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
            return Err(ApiError::InvalidUrl(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        Ok(())
    }

    pub async fn delete_realm(&self, realm: &str) -> Result<(), ApiError> {
        let url = format!("{}/api2/json/access/domains/{}", self.base_url, realm);
        tracing::debug!("Deleting realm at: {}", url);

        let response = self
            .http
            .delete(url)
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
            return Err(ApiError::InvalidUrl(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealmConfig {
    pub realm: String,
    #[serde(rename = "type")]
    pub realm_type: String,
    #[serde(rename = "issuer-url")]
    pub issuer_url: String,
    #[serde(rename = "client-id")]
    pub client_id: String,
    #[serde(rename = "client-key")]
    pub client_key: String,
    #[serde(rename = "username-claim", skip_serializing_if = "Option::is_none")]
    pub username_claim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocreate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(rename = "groups-overwrite", skip_serializing_if = "Option::is_none")]
    pub groups_overwrite: Option<bool>,
    #[serde(rename = "groups-autocreate", skip_serializing_if = "Option::is_none")]
    pub groups_autocreate: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RealmInfo {
    #[serde(rename = "type")]
    pub realm_type: String,
    #[serde(rename = "issuer-url")]
    pub issuer_url: String,
    #[serde(rename = "client-id")]
    pub client_id: String,
    #[serde(rename = "username-claim")]
    pub username_claim: Option<String>,
    pub autocreate: Option<u8>,
    pub default: Option<u8>,
    pub comment: Option<String>,
    #[serde(rename = "groups-overwrite")]
    pub groups_overwrite: Option<u8>,
    #[serde(rename = "groups-autocreate")]
    pub groups_autocreate: Option<u8>,
    #[serde(skip)]
    pub digest: Option<String>,
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

    #[tokio::test]
    async fn client_creates_realm_successfully() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api2/json/access/domains")
            .match_header("authorization", "PVEAPIToken=user@realm!token=secret")
            .with_status(200)
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let client =
            Client::new(server.url(), "user@realm!token=secret".to_string(), true).unwrap();

        let config = RealmConfig {
            realm: "authentik".to_string(),
            realm_type: "openid".to_string(),
            issuer_url: "https://auth.example.com".to_string(),
            client_id: "proxmox".to_string(),
            client_key: "secret123".to_string(),
            username_claim: Some("username".to_string()),
            autocreate: Some(true),
            default: Some(true),
            comment: None,
            groups_autocreate: Some(true),
            groups_overwrite: Some(true),
        };

        let result = client.create_realm(config).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn client_checks_realm_exists() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api2/json/access/domains/authentik")
            .match_header("authorization", "PVEAPIToken=user@realm!token=secret")
            .with_status(200)
            .with_body(
                r#"{
                "data": {
                    "type": "openid",
                    "issuer-url": "https://auth.example.com",
                    "client-id": "proxmox",
                    "username-claim": "username",
                    "autocreate": 1,
                    "default": 1
                }
            }"#,
            )
            .create_async()
            .await;

        let client =
            Client::new(server.url(), "user@realm!token=secret".to_string(), true).unwrap();

        let result = client.get_realm("authentik").await;
        assert!(result.is_ok());

        let realm_info = result.unwrap();
        assert!(realm_info.is_some());

        let info = realm_info.unwrap();
        assert_eq!(info.realm_type, "openid");
        assert_eq!(info.issuer_url, "https://auth.example.com");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn client_handles_realm_not_found() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api2/json/access/domains/nonexistent")
            .match_header("authorization", "PVEAPIToken=user@realm!token=secret")
            .with_status(500)
            .with_body(r#"{"errors":{"realm":"domain 'nonexistent' does not exist"}}"#)
            .create_async()
            .await;

        let client =
            Client::new(server.url(), "user@realm!token=secret".to_string(), true).unwrap();

        let result = client.get_realm("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn client_deletes_realm_successfully() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/api2/json/access/domains/authentik")
            .match_header("authorization", "PVEAPIToken=user@realm!token=secret")
            .with_status(200)
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let client =
            Client::new(server.url(), "user@realm!token=secret".to_string(), true).unwrap();

        let result = client.delete_realm("authentik").await;
        assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn client_updates_realm_successfully() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api2/json/access/domains/authentik")
            .match_header("authorization", "PVEAPIToken=user@realm!token=secret")
            .with_status(200)
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let client =
            Client::new(server.url(), "user@realm!token=secret".to_string(), true).unwrap();

        let config = RealmConfig {
            realm: "authentik".to_string(),
            realm_type: "openid".to_string(),
            issuer_url: "https://new-auth.example.com".to_string(),
            client_id: "new-proxmox".to_string(),
            client_key: "newsecret".to_string(),
            username_claim: None,
            autocreate: None,
            default: Some(false),
            comment: None,
            groups_autocreate: None,
            groups_overwrite: None,
        };

        let result = client.update_realm("authentik", config).await;
        assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn client_handles_realm_creation_conflict() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api2/json/access/domains")
            .with_status(400)
            .with_body(r#"{"errors":{"realm":"domain 'authentik' already exists"}}"#)
            .create_async()
            .await;

        let client =
            Client::new(server.url(), "user@realm!token=secret".to_string(), true).unwrap();

        let config = RealmConfig {
            realm: "authentik".to_string(),
            realm_type: "openid".to_string(),
            issuer_url: "https://auth.example.com".to_string(),
            client_id: "proxmox".to_string(),
            client_key: "secret123".to_string(),
            username_claim: None,
            autocreate: None,
            default: None,
            comment: None,
            groups_autocreate: None,
            groups_overwrite: None,
        };

        let result = client.create_realm(config).await;
        assert!(result.is_err());

        mock.assert_async().await;
    }
}
