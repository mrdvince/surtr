//! Provider trait and related types
//!
//! This module defines the main Provider trait that all Terraform providers
//! must implement, along with associated request/response types.

use crate::context::Context;
use crate::schema::Schema;
use crate::types::{ClientCapabilities, Diagnostic, DynamicValue, ServerCapabilities};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Provider is the main entry point - implement this to create a provider
/// Lifecycle: new() -> configure() -> resources/data_sources called
#[async_trait]
pub trait Provider: Send + Sync {
    /// Return type name (e.g., "proxmox") - MUST be constant
    fn type_name(&self) -> &str;

    /// Called first to get provider capabilities
    async fn metadata(
        &self,
        ctx: Context,
        request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse;

    /// Called to get provider configuration schema
    async fn schema(&self, ctx: Context, request: ProviderSchemaRequest) -> ProviderSchemaResponse;

    /// Called to get provider meta-schema (for provider_meta blocks)
    async fn meta_schema(
        &self,
        ctx: Context,
        request: ProviderMetaSchemaRequest,
    ) -> ProviderMetaSchemaResponse;

    /// Called after validation to configure the provider - store clients here
    /// IMPORTANT: provider_data from response is passed to all resources/data sources
    async fn configure(
        &mut self,
        ctx: Context,
        request: ConfigureProviderRequest,
    ) -> ConfigureProviderResponse;

    /// Called to validate provider configuration
    async fn validate(
        &self,
        ctx: Context,
        request: ValidateProviderConfigRequest,
    ) -> ValidateProviderConfigResponse;

    /// Called when provider is shutting down
    async fn stop(&self, ctx: Context, request: StopProviderRequest) -> StopProviderResponse;

    /// Return resource factories - these create new instances on each call
    /// CRITICAL: Factories MUST return ResourceWithConfigure trait objects
    fn resources(&self) -> HashMap<String, ResourceFactory>;

    /// Return data source factories - these create new instances on each call
    /// CRITICAL: Factories MUST return DataSourceWithConfigure trait objects
    fn data_sources(&self) -> HashMap<String, DataSourceFactory>;
}

/// Factory type for creating resources
/// CRITICAL: Must return ResourceWithConfigure (not base Resource trait)
pub type ResourceFactory =
    Box<dyn Fn() -> Box<dyn crate::resource::ResourceWithConfigure> + Send + Sync>;

/// Factory type for creating data sources  
/// CRITICAL: Must return DataSourceWithConfigure (not base DataSource trait)
pub type DataSourceFactory =
    Box<dyn Fn() -> Box<dyn crate::data_source::DataSourceWithConfigure> + Send + Sync>;

// Request/Response types

/// Request for provider metadata
pub struct ProviderMetadataRequest;

/// Response with provider metadata
pub struct ProviderMetadataResponse {
    pub type_name: String,
    pub server_capabilities: ServerCapabilities,
}

/// Request for provider schema
pub struct ProviderSchemaRequest;

/// Response with provider schema
pub struct ProviderSchemaResponse {
    pub schema: Schema,
    pub diagnostics: Vec<Diagnostic>,
}

/// Request for provider meta-schema
pub struct ProviderMetaSchemaRequest;

/// Response with provider meta-schema
pub struct ProviderMetaSchemaResponse {
    pub schema: Option<Schema>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Request to configure provider
pub struct ConfigureProviderRequest {
    pub terraform_version: String,
    pub config: DynamicValue,
    pub client_capabilities: ClientCapabilities,
}

/// Response from provider configuration
pub struct ConfigureProviderResponse {
    pub diagnostics: Vec<Diagnostic>,
    /// Provider-specific data passed to all resources/data sources
    /// Typically contains API clients, credentials, etc.
    /// IMPORTANT: This is what gets passed to ResourceWithConfigure.configure()
    pub provider_data: Option<Arc<dyn Any + Send + Sync>>,
}

/// Request to validate provider config
pub struct ValidateProviderConfigRequest {
    pub config: DynamicValue,
    pub client_capabilities: ClientCapabilities,
}

/// Response from provider validation
pub struct ValidateProviderConfigResponse {
    pub diagnostics: Vec<Diagnostic>,
}

/// Request to stop provider
pub struct StopProviderRequest;

/// Response from stopping provider
pub struct StopProviderResponse {
    pub error: Option<String>,
}

/// Optional trait for providers with functions
#[async_trait]
pub trait ProviderWithFunctions: Provider {
    /// Return function factories
    fn functions(&self) -> HashMap<String, FunctionFactory>;
}

/// Factory type for creating functions
pub type FunctionFactory = Box<dyn Fn() -> Box<dyn crate::function::Function> + Send + Sync>;

/// Optional trait for providers with ephemeral resources
#[async_trait]
pub trait ProviderWithEphemeralResources: Provider {
    /// Return ephemeral resource factories
    fn ephemeral_resources(&self) -> HashMap<String, EphemeralResourceFactory>;
}

/// Factory type for creating ephemeral resources
pub type EphemeralResourceFactory =
    Box<dyn Fn() -> Box<dyn crate::ephemeral::EphemeralResource> + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider {
        configured: bool,
    }

    #[async_trait]
    impl Provider for TestProvider {
        fn type_name(&self) -> &str {
            "test"
        }

        async fn metadata(
            &self,
            _ctx: Context,
            _request: ProviderMetadataRequest,
        ) -> ProviderMetadataResponse {
            ProviderMetadataResponse {
                type_name: "test".to_string(),
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
            ProviderSchemaResponse {
                schema: crate::schema::SchemaBuilder::new().build(),
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
            _request: ConfigureProviderRequest,
        ) -> ConfigureProviderResponse {
            self.configured = true;
            ConfigureProviderResponse {
                diagnostics: vec![],
                provider_data: Some(Arc::new("test_data".to_string()) as Arc<dyn Any + Send + Sync>),
            }
        }

        async fn validate(
            &self,
            _ctx: Context,
            _request: ValidateProviderConfigRequest,
        ) -> ValidateProviderConfigResponse {
            ValidateProviderConfigResponse {
                diagnostics: vec![],
            }
        }

        async fn stop(&self, _ctx: Context, _request: StopProviderRequest) -> StopProviderResponse {
            StopProviderResponse { error: None }
        }

        fn resources(&self) -> HashMap<String, ResourceFactory> {
            HashMap::new()
        }

        fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
            HashMap::new()
        }
    }

    #[tokio::test]
    async fn provider_metadata_returns_type_name() {
        let provider = TestProvider { configured: false };
        let ctx = Context::new();
        let response = provider.metadata(ctx, ProviderMetadataRequest).await;

        assert_eq!(response.type_name, "test");
        assert!(!response.server_capabilities.plan_destroy);
    }

    #[tokio::test]
    async fn provider_configure_stores_data() {
        let mut provider = TestProvider { configured: false };
        let ctx = Context::new();
        let request = ConfigureProviderRequest {
            terraform_version: "1.0.0".to_string(),
            config: DynamicValue::null(),
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };

        let response = provider.configure(ctx, request).await;

        assert!(provider.configured);
        assert!(response.diagnostics.is_empty());
        assert!(response.provider_data.is_some());
    }
}
