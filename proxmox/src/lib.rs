pub mod api;
pub mod data_sources;
pub mod resources;

use async_trait::async_trait;
use std::collections::HashMap;
use tfplug::provider::{DataSourceSchema, ResourceSchema};
use tfplug::request::{ConfigureRequest, ConfigureResponse};
use tfplug::{DataSourceV2, Diagnostics, ProviderV2, ResourceV2};

pub struct ProxmoxProvider {
    client: Option<api::Client>,
}

impl Default for ProxmoxProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxmoxProvider {
    pub fn new() -> Self {
        Self { client: None }
    }
}

#[async_trait]
impl ProviderV2 for ProxmoxProvider {
    async fn configure(&mut self, request: ConfigureRequest) -> ConfigureResponse {
        let endpoint = request
            .config
            .values
            .get("endpoint")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("PROXMOX_ENDPOINT").ok());

        let api_token = request
            .config
            .values
            .get("api_token")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("PROXMOX_API_TOKEN").ok());

        let insecure = request
            .config
            .values
            .get("insecure")
            .and_then(|v| v.as_bool())
            .or_else(|| {
                std::env::var("PROXMOX_INSECURE")
                    .ok()
                    .and_then(|v| v.parse::<bool>().ok())
            })
            .unwrap_or(false);

        let mut diags = Diagnostics::new();

        match (endpoint, api_token) {
            (Some(endpoint), Some(api_token)) => {
                match api::Client::new(endpoint.clone(), api_token.clone(), insecure) {
                    Ok(client) => {
                        self.client = Some(client);
                    }
                    Err(e) => {
                        diags.add_error(
                            format!("Failed to create API client: {}", e),
                            None::<String>,
                        );
                    }
                }
            }
            (None, _) => {
                diags.add_error(
                    "endpoint is required (set in provider config or PROXMOX_ENDPOINT env var)",
                    None::<String>,
                );
            }
            (_, None) => {
                diags.add_error(
                    "api_token is required (set in provider config or PROXMOX_API_TOKEN env var)",
                    None::<String>,
                );
            }
        }

        ConfigureResponse { diagnostics: diags }
    }

    async fn create_resource(&self, name: &str) -> tfplug::Result<Box<dyn ResourceV2>> {
        let client = self
            .client
            .as_ref()
            .ok_or("Provider not configured")?
            .clone();

        match name {
            "proxmox_realm" => Ok(Box::new(resources::realm::RealmResource::new(client))),
            _ => Err(format!("Unknown resource: {}", name).into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> tfplug::Result<Box<dyn DataSourceV2>> {
        let client = self
            .client
            .as_ref()
            .ok_or("Provider not configured")?
            .clone();

        match name {
            "proxmox_version" => Ok(Box::new(data_sources::version::VersionDataSource::new(
                client,
            ))),
            _ => Err(format!("Unknown data source: {}", name).into()),
        }
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        static SCHEMAS: std::sync::OnceLock<HashMap<String, ResourceSchema>> =
            std::sync::OnceLock::new();

        SCHEMAS
            .get_or_init(|| {
                let mut schemas = HashMap::new();
                schemas.insert(
                    "proxmox_realm".to_string(),
                    resources::RealmResource::schema_static(),
                );
                schemas
            })
            .clone()
    }

    async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema> {
        static SCHEMAS: std::sync::OnceLock<HashMap<String, DataSourceSchema>> =
            std::sync::OnceLock::new();

        SCHEMAS
            .get_or_init(|| {
                let mut schemas = HashMap::new();
                schemas.insert(
                    "proxmox_version".to_string(),
                    data_sources::VersionDataSource::schema_static(),
                );
                schemas
            })
            .clone()
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tfplug::context::Context;
    use tfplug::types::Config;

    #[tokio::test]
    #[serial]
    async fn provider_configures_successfully_with_env_vars() {
        std::env::set_var("PROXMOX_ENDPOINT", "https://localhost:8006");
        std::env::set_var("PROXMOX_API_TOKEN", "test@pve!token=secret");
        std::env::set_var("PROXMOX_INSECURE", "true");

        let mut provider = ProxmoxProvider::new();
        let request = ConfigureRequest {
            context: Context::new(),
            config: Config {
                values: HashMap::new(),
            },
        };

        let response = provider.configure(request).await;
        assert!(response.diagnostics.errors.is_empty());
        assert!(provider.client.is_some());

        std::env::remove_var("PROXMOX_ENDPOINT");
        std::env::remove_var("PROXMOX_API_TOKEN");
        std::env::remove_var("PROXMOX_INSECURE");
    }

    #[tokio::test]
    #[serial]
    async fn provider_configure_requires_endpoint() {
        std::env::remove_var("PROXMOX_ENDPOINT");
        std::env::set_var("PROXMOX_API_TOKEN", "test@pve!token=secret");

        let mut provider = ProxmoxProvider::new();
        let request = ConfigureRequest {
            context: Context::new(),
            config: Config {
                values: HashMap::new(),
            },
        };

        let response = provider.configure(request).await;
        assert!(!response.diagnostics.errors.is_empty());
        assert!(response.diagnostics.errors[0]
            .summary
            .contains("endpoint is required"));

        std::env::remove_var("PROXMOX_API_TOKEN");
    }

    #[tokio::test]
    #[serial]
    async fn provider_configure_requires_api_token() {
        std::env::set_var("PROXMOX_ENDPOINT", "https://localhost:8006");
        std::env::remove_var("PROXMOX_API_TOKEN");

        let mut provider = ProxmoxProvider::new();
        let request = ConfigureRequest {
            context: Context::new(),
            config: Config {
                values: HashMap::new(),
            },
        };

        let response = provider.configure(request).await;
        assert!(!response.diagnostics.errors.is_empty());
        assert!(response.diagnostics.errors[0]
            .summary
            .contains("api_token is required"));

        std::env::remove_var("PROXMOX_ENDPOINT");
    }

    #[tokio::test]
    #[serial]
    async fn provider_creates_resources_after_configuration() {
        std::env::set_var("PROXMOX_ENDPOINT", "https://localhost:8006");
        std::env::set_var("PROXMOX_API_TOKEN", "test@pve!token=secret");
        std::env::set_var("PROXMOX_INSECURE", "true");

        let mut provider = ProxmoxProvider::new();
        let request = ConfigureRequest {
            context: Context::new(),
            config: Config {
                values: HashMap::new(),
            },
        };

        provider.configure(request).await;

        let resource = provider.create_resource("proxmox_realm").await;
        assert!(resource.is_ok());

        let unknown_resource = provider.create_resource("unknown_resource").await;
        assert!(unknown_resource.is_err());

        std::env::remove_var("PROXMOX_ENDPOINT");
        std::env::remove_var("PROXMOX_API_TOKEN");
        std::env::remove_var("PROXMOX_INSECURE");
    }

    #[tokio::test]
    #[serial]
    async fn provider_creates_data_sources_after_configuration() {
        std::env::set_var("PROXMOX_ENDPOINT", "https://localhost:8006");
        std::env::set_var("PROXMOX_API_TOKEN", "test@pve!token=secret");
        std::env::set_var("PROXMOX_INSECURE", "true");

        let mut provider = ProxmoxProvider::new();
        let request = ConfigureRequest {
            context: Context::new(),
            config: Config {
                values: HashMap::new(),
            },
        };

        provider.configure(request).await;

        let data_source = provider.create_data_source("proxmox_version").await;
        assert!(data_source.is_ok());

        let unknown_data_source = provider.create_data_source("unknown_data_source").await;
        assert!(unknown_data_source.is_err());

        std::env::remove_var("PROXMOX_ENDPOINT");
        std::env::remove_var("PROXMOX_API_TOKEN");
        std::env::remove_var("PROXMOX_INSECURE");
    }

    #[tokio::test]
    async fn provider_fails_to_create_resources_before_configuration() {
        let provider = ProxmoxProvider::new();

        let resource = provider.create_resource("proxmox_realm").await;
        assert!(resource.is_err());
        assert!(resource
            .err()
            .unwrap()
            .to_string()
            .contains("Provider not configured"));
    }

    #[tokio::test]
    async fn provider_schemas_are_cached_and_immutable() {
        let provider = ProxmoxProvider::new();

        let schemas1 = provider.resource_schemas().await;
        let schemas2 = provider.resource_schemas().await;

        assert_eq!(schemas1.len(), schemas2.len());

        let data_schemas1 = provider.data_source_schemas().await;
        let data_schemas2 = provider.data_source_schemas().await;

        assert_eq!(data_schemas1.len(), data_schemas2.len());
    }

    #[tokio::test]
    async fn provider_schemas_contain_expected_resources() {
        let provider = ProxmoxProvider::new();

        let resource_schemas = provider.resource_schemas().await;
        assert!(resource_schemas.contains_key("proxmox_realm"));

        let data_source_schemas = provider.data_source_schemas().await;
        assert!(data_source_schemas.contains_key("proxmox_version"));
    }
}
