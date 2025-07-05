//! Proxmox Terraform Provider
//!
//! This provider enables management of Proxmox VE resources through Terraform.

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tfplug::context::Context;
use tfplug::provider::{
    ConfigureProviderRequest, ConfigureProviderResponse, DataSourceFactory, Provider,
    ProviderMetaSchemaRequest, ProviderMetaSchemaResponse, ProviderMetadataRequest,
    ProviderMetadataResponse, ProviderSchemaRequest, ProviderSchemaResponse, ResourceFactory,
    StopProviderRequest, StopProviderResponse, ValidateProviderConfigRequest,
    ValidateProviderConfigResponse,
};
use tfplug::schema::{AttributeBuilder, AttributeType, SchemaBuilder};
use tfplug::types::{AttributePath, Diagnostic, ServerCapabilities};

pub mod api;
pub mod data_sources;
mod provider_data;
pub mod resources;

pub use provider_data::ProxmoxProviderData;

/// Main Proxmox provider struct
pub struct ProxmoxProvider {
    /// API client instance (set during configure)
    client: Option<api::Client>,
}

impl ProxmoxProvider {
    pub fn new() -> Self {
        Self { client: None }
    }
}

impl Default for ProxmoxProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for ProxmoxProvider {
    fn type_name(&self) -> &str {
        "proxmox"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse {
        ProviderMetadataResponse {
            type_name: self.type_name().to_string(),
            server_capabilities: ServerCapabilities {
                plan_destroy: false,
                get_provider_schema_optional: false,
                move_resource_state: false,
            },
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ProviderSchemaRequest,
    ) -> ProviderSchemaResponse {
        let schema = SchemaBuilder::new()
            .version(0)
            .description("Proxmox VE provider configuration")
            .attribute(
                AttributeBuilder::new("endpoint", AttributeType::String)
                    .description("The API endpoint URL (e.g., https://proxmox.example.com:8006)")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("api_token", AttributeType::String)
                    .description("API token for authentication (format: user@realm!tokenid=secret)")
                    .optional()
                    .sensitive()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("insecure", AttributeType::Bool)
                    .description("Skip TLS certificate verification")
                    .optional()
                    .build(),
            )
            .build();

        ProviderSchemaResponse {
            schema,
            diagnostics: vec![],
        }
    }

    async fn meta_schema(
        &self,
        _ctx: Context,
        _request: ProviderMetaSchemaRequest,
    ) -> ProviderMetaSchemaResponse {
        ProviderMetaSchemaResponse {
            schema: None,
            diagnostics: vec![],
        }
    }

    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureProviderRequest,
    ) -> ConfigureProviderResponse {
        let mut diagnostics = Vec::new();

        let endpoint = request
            .config
            .get_string(&AttributePath::new("endpoint"))
            .ok()
            .or_else(|| std::env::var("PROXMOX_ENDPOINT").ok());

        let api_token = request
            .config
            .get_string(&AttributePath::new("api_token"))
            .ok()
            .or_else(|| std::env::var("PROXMOX_API_TOKEN").ok());

        let insecure = request
            .config
            .get_bool(&AttributePath::new("insecure"))
            .unwrap_or_else(|_| {
                std::env::var("PROXMOX_INSECURE")
                    .ok()
                    .map(|s| s.to_lowercase() == "true")
                    .unwrap_or(false)
            });

        let endpoint = match endpoint {
            Some(e) => e,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Missing endpoint",
                    "The 'endpoint' configuration is required. Set it in the provider config or PROXMOX_ENDPOINT environment variable.",
                ));
                return ConfigureProviderResponse {
                    diagnostics,
                    provider_data: None,
                };
            }
        };

        let api_token = match api_token {
            Some(t) => t,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Missing API token",
                    "The 'api_token' configuration is required. Set it in the provider config or PROXMOX_API_TOKEN environment variable.",
                ));
                return ConfigureProviderResponse {
                    diagnostics,
                    provider_data: None,
                };
            }
        };

        match api::Client::new(&endpoint, &api_token, insecure) {
            Ok(client) => {
                let provider_data = ProxmoxProviderData::new(client.clone());
                self.client = Some(client);
                ConfigureProviderResponse {
                    diagnostics,
                    provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Failed to create API client",
                    format!("Error: {}", e),
                ));
                ConfigureProviderResponse {
                    diagnostics,
                    provider_data: None,
                }
            }
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        request: ValidateProviderConfigRequest,
    ) -> ValidateProviderConfigResponse {
        let mut diagnostics = Vec::new();

        if let Ok(endpoint) = request.config.get_string(&AttributePath::new("endpoint")) {
            if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
                diagnostics.push(Diagnostic::error(
                    "Invalid endpoint",
                    "The endpoint must start with http:// or https://",
                ));
            }
        }

        if let Ok(api_token) = request.config.get_string(&AttributePath::new("api_token")) {
            if !api_token.contains('!') || !api_token.contains('=') {
                diagnostics.push(Diagnostic::warning(
                    "Invalid API token format",
                    "API token should be in format: user@realm!tokenid=secret",
                ));
            }
        }

        ValidateProviderConfigResponse { diagnostics }
    }

    async fn stop(&self, _ctx: Context, _request: StopProviderRequest) -> StopProviderResponse {
        StopProviderResponse { error: None }
    }

    fn resources(&self) -> HashMap<String, ResourceFactory> {
        let mut resources = HashMap::new();

        resources.insert(
            "proxmox_realm".to_string(),
            Box::new(|| {
                Box::new(resources::RealmResource::new()) as Box<dyn tfplug::ResourceWithConfigure>
            }) as ResourceFactory,
        );

        resources.insert(
            "proxmox_qemu_vm".to_string(),
            Box::new(|| {
                Box::new(resources::QemuVmResource::new()) as Box<dyn tfplug::ResourceWithConfigure>
            }) as ResourceFactory,
        );

        resources
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
        let mut data_sources = HashMap::new();

        data_sources.insert(
            "proxmox_version".to_string(),
            Box::new(|| {
                Box::new(data_sources::data_source_version::VersionDataSource::new())
                    as Box<dyn tfplug::DataSourceWithConfigure>
            }) as DataSourceFactory,
        );

        data_sources
    }
}
