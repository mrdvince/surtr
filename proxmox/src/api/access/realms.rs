//! Realm (authentication domain) API implementation

use super::super::common::{deserialize_proxmox_bool_option, ProxmoxApiResource};
use serde::{Deserialize, Serialize};

/// Type alias for realm configuration
pub type Realm = RealmConfig;

/// Realm configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmConfig {
    pub realm: String,
    #[serde(rename = "type")]
    pub realm_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,

    // OpenID specific fields
    #[serde(rename = "issuer-url", skip_serializing_if = "Option::is_none")]
    pub issuer_url: Option<String>,
    #[serde(rename = "client-id", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(rename = "client-key", skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
    #[serde(rename = "username-claim", skip_serializing_if = "Option::is_none")]
    pub username_claim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocreate: Option<bool>,
    #[serde(rename = "groups-overwrite", skip_serializing_if = "Option::is_none")]
    pub groups_overwrite: Option<bool>,
    #[serde(rename = "groups-autocreate", skip_serializing_if = "Option::is_none")]
    pub groups_autocreate: Option<bool>,
}

/// Response from GET /api2/json/access/domains/{realm}
#[derive(Debug, Deserialize)]
struct GetRealmResponse {
    #[serde(rename = "type")]
    realm_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_proxmox_bool_option",
        default
    )]
    default: Option<bool>,

    // OpenID specific fields
    #[serde(rename = "issuer-url", skip_serializing_if = "Option::is_none")]
    issuer_url: Option<String>,
    #[serde(rename = "client-id", skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(rename = "client-key", skip_serializing_if = "Option::is_none")]
    client_key: Option<String>,
    #[serde(rename = "username-claim", skip_serializing_if = "Option::is_none")]
    username_claim: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_proxmox_bool_option",
        default
    )]
    autocreate: Option<bool>,
    #[serde(
        rename = "groups-overwrite",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_proxmox_bool_option",
        default
    )]
    groups_overwrite: Option<bool>,
    #[serde(
        rename = "groups-autocreate",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_proxmox_bool_option",
        default
    )]
    groups_autocreate: Option<bool>,

    // Extra field from API
    #[allow(dead_code)]
    digest: Option<String>,
}

/// Request body for creating realms
#[derive(Debug, Serialize)]
pub struct CreateRealmRequest {
    pub realm: String,
    #[serde(rename = "type")]
    pub realm_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,

    // OpenID specific fields
    #[serde(rename = "issuer-url", skip_serializing_if = "Option::is_none")]
    pub issuer_url: Option<String>,
    #[serde(rename = "client-id", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(rename = "client-key", skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
    #[serde(rename = "username-claim", skip_serializing_if = "Option::is_none")]
    pub username_claim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocreate: Option<bool>,
    #[serde(rename = "groups-overwrite", skip_serializing_if = "Option::is_none")]
    pub groups_overwrite: Option<bool>,
    #[serde(rename = "groups-autocreate", skip_serializing_if = "Option::is_none")]
    pub groups_autocreate: Option<bool>,
}

/// Request body for updating realms
#[derive(Debug, Serialize)]
pub struct UpdateRealmRequest {
    #[serde(rename = "type")]
    pub realm_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,

    // OpenID specific fields
    #[serde(rename = "issuer-url", skip_serializing_if = "Option::is_none")]
    pub issuer_url: Option<String>,
    #[serde(rename = "client-id", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(rename = "client-key", skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
    #[serde(rename = "username-claim", skip_serializing_if = "Option::is_none")]
    pub username_claim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocreate: Option<bool>,
    #[serde(rename = "groups-overwrite", skip_serializing_if = "Option::is_none")]
    pub groups_overwrite: Option<bool>,
    #[serde(rename = "groups-autocreate", skip_serializing_if = "Option::is_none")]
    pub groups_autocreate: Option<bool>,
}

impl ProxmoxApiResource for RealmConfig {
    type CreateRequest = CreateRealmRequest;
    type UpdateRequest = UpdateRealmRequest;

    fn api_path() -> &'static str {
        "/api2/json/access/domains"
    }
}

impl super::super::Client {
    /// List all realms
    pub async fn list_realms(&self) -> Result<Vec<RealmConfig>, super::super::ApiError> {
        self.get(RealmConfig::api_path()).await
    }

    /// Get a specific realm
    pub async fn get_realm(&self, realm: &str) -> Result<RealmConfig, super::super::ApiError> {
        let path = RealmConfig::resource_path(realm);
        let response: GetRealmResponse = self.get(&path).await?;

        Ok(RealmConfig {
            realm: realm.to_string(),
            realm_type: response.realm_type,
            comment: response.comment,
            default: response.default,
            issuer_url: response.issuer_url,
            client_id: response.client_id,
            client_key: response.client_key,
            username_claim: response.username_claim,
            autocreate: response.autocreate,
            groups_overwrite: response.groups_overwrite,
            groups_autocreate: response.groups_autocreate,
        })
    }

    /// Create a new realm
    pub async fn create_realm(&self, config: &RealmConfig) -> Result<(), super::super::ApiError> {
        let path = RealmConfig::api_path();

        let request = CreateRealmRequest {
            realm: config.realm.clone(),
            realm_type: config.realm_type.clone(),
            comment: config.comment.clone(),
            default: config.default,
            issuer_url: config.issuer_url.clone(),
            client_id: config.client_id.clone(),
            client_key: config.client_key.clone(),
            username_claim: config.username_claim.clone(),
            autocreate: config.autocreate,
            groups_overwrite: config.groups_overwrite,
            groups_autocreate: config.groups_autocreate,
        };

        self.post::<(), _>(path, &request).await.map(|_| ())
    }

    /// Update an existing realm
    pub async fn update_realm(&self, config: &RealmConfig) -> Result<(), super::super::ApiError> {
        let path = RealmConfig::resource_path(&config.realm);

        let request = UpdateRealmRequest {
            realm_type: config.realm_type.clone(),
            comment: config.comment.clone(),
            default: config.default,
            issuer_url: config.issuer_url.clone(),
            client_id: config.client_id.clone(),
            client_key: config.client_key.clone(),
            username_claim: config.username_claim.clone(),
            autocreate: config.autocreate,
            groups_overwrite: config.groups_overwrite,
            groups_autocreate: config.groups_autocreate,
        };

        self.put::<(), _>(&path, &request).await.map(|_| ())
    }

    /// Delete a realm
    pub async fn delete_realm(&self, realm: &str) -> Result<(), super::super::ApiError> {
        let path = RealmConfig::resource_path(realm);
        self.delete::<()>(&path).await.map(|_| ())
    }
}

/// Realms API for realm operations
pub struct RealmsApi<'a> {
    client: &'a super::super::Client,
}

impl<'a> RealmsApi<'a> {
    pub fn new(client: &'a super::super::Client) -> Self {
        Self { client }
    }

    /// GET /api2/json/access/domains
    pub async fn list(&self) -> Result<Vec<Realm>, super::super::ApiError> {
        self.client.get("/api2/json/access/domains").await
    }

    /// GET /api2/json/access/domains/{realm}
    pub async fn get(&self, realm: &str) -> Result<Realm, super::super::ApiError> {
        let path = RealmConfig::resource_path(realm);
        let response: GetRealmResponse = self.client.get(&path).await?;

        Ok(RealmConfig {
            realm: realm.to_string(),
            realm_type: response.realm_type,
            comment: response.comment,
            default: response.default,
            issuer_url: response.issuer_url,
            client_id: response.client_id,
            client_key: response.client_key,
            username_claim: response.username_claim,
            autocreate: response.autocreate,
            groups_overwrite: response.groups_overwrite,
            groups_autocreate: response.groups_autocreate,
        })
    }

    /// POST /api2/json/access/domains
    pub async fn create(&self, request: &CreateRealmRequest) -> Result<(), super::super::ApiError> {
        self.client
            .post::<(), _>("/api2/json/access/domains", request)
            .await
            .map(|_| ())
    }

    /// PUT /api2/json/access/domains/{realm}
    pub async fn update(
        &self,
        realm: &str,
        request: &UpdateRealmRequest,
    ) -> Result<(), super::super::ApiError> {
        self.client
            .put::<(), _>(&format!("/api2/json/access/domains/{}", realm), request)
            .await
            .map(|_| ())
    }

    /// DELETE /api2/json/access/domains/{realm}
    pub async fn delete(&self, realm: &str) -> Result<(), super::super::ApiError> {
        self.client
            .delete::<()>(&format!("/api2/json/access/domains/{}", realm))
            .await
            .map(|_| ())
    }
}
