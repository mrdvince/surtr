//! Advanced example showing the new tfplug framework API with sophisticated features
//!
//! This example demonstrates:
//! - Multiple resources and data sources
//! - Provider configuration validation
//! - Resource import functionality
//! - Plan modification with replacement detection
//! - State upgrade from older versions
//! - Complex schema definitions with validators
//! - Concurrent operations

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tfplug::context::Context;
use tfplug::data_source::{
    ConfigureDataSourceRequest, ConfigureDataSourceResponse, DataSource, DataSourceMetadataRequest,
    DataSourceMetadataResponse, DataSourceSchemaRequest, DataSourceSchemaResponse,
    DataSourceWithConfigure, ReadDataSourceRequest, ReadDataSourceResponse,
    ValidateDataSourceConfigRequest, ValidateDataSourceConfigResponse,
};
use tfplug::defaults::StaticDefault;
use tfplug::import::import_state_passthrough_id;
use tfplug::plan_modifier::{RequiresReplace, UseStateForUnknown};
use tfplug::provider::*;
use tfplug::resource::{
    ConfigureResourceRequest, ConfigureResourceResponse, CreateResourceRequest,
    CreateResourceResponse, DeleteResourceRequest, DeleteResourceResponse,
    ImportResourceStateRequest, ImportResourceStateResponse, ModifyPlanRequest, ModifyPlanResponse,
    ReadResourceRequest, ReadResourceResponse, Resource, ResourceMetadataRequest,
    ResourceMetadataResponse, ResourceSchemaRequest, ResourceSchemaResponse, ResourceWithConfigure,
    ResourceWithImportState, ResourceWithModifyPlan, ResourceWithUpgradeState,
    UpdateResourceRequest, UpdateResourceResponse, UpgradeResourceStateRequest,
    UpgradeResourceStateResponse, ValidateResourceConfigRequest, ValidateResourceConfigResponse,
};
use tfplug::schema::AttributeType;
use tfplug::types::{
    AttributePath, ClientCapabilities, Diagnostic, Dynamic, DynamicValue, ServerCapabilities,
};
use tfplug::validator::{StringLengthValidator, StringOneOfValidator};
use tfplug::{AttributeBuilder, SchemaBuilder};
use tokio::sync::RwLock;

// Shared provider configuration
#[derive(Clone)]
#[allow(dead_code)]
struct ProviderConfig {
    api_endpoint: String,
    api_key: String,
    environment: String,
}

// API client wrapper
#[derive(Clone)]
struct ApiClient {
    config: ProviderConfig,
}

impl ApiClient {
    fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    async fn create_server(&self, name: &str, size: &str) -> Result<String, String> {
        // Simulate API call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!(
            "Creating server '{}' of size '{}' in {}",
            name, size, self.config.api_endpoint
        );
        Ok(format!("srv-{}", uuid::Uuid::new_v4()))
    }

    async fn get_server(&self, id: &str) -> Result<Option<ServerInfo>, String> {
        // Simulate API call
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        Ok(Some(ServerInfo {
            id: id.to_string(),
            name: "test-server".to_string(),
            size: "medium".to_string(),
            status: "running".to_string(),
            ip_address: "10.0.0.1".to_string(),
        }))
    }

    async fn update_server(&self, id: &str, name: &str, size: &str) -> Result<(), String> {
        // Simulate API call
        println!("Updating server '{}': name='{}', size='{}'", id, name, size);
        Ok(())
    }

    async fn delete_server(&self, id: &str) -> Result<(), String> {
        // Simulate API call
        println!("Deleting server '{}'", id);
        Ok(())
    }

    async fn get_zone_info(&self, zone: &str) -> Result<ZoneInfo, String> {
        // Simulate API call
        Ok(ZoneInfo {
            zone: zone.to_string(),
            region: "us-east".to_string(),
            available_sizes: vec!["small", "medium", "large"],
        })
    }
}

#[allow(dead_code)]
struct ServerInfo {
    id: String,
    name: String,
    size: String,
    status: String,
    ip_address: String,
}

struct ZoneInfo {
    zone: String,
    region: String,
    available_sizes: Vec<&'static str>,
}

// Provider implementation
struct AdvancedProvider {
    client: Arc<RwLock<Option<ApiClient>>>,
}

impl AdvancedProvider {
    fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Provider for AdvancedProvider {
    fn type_name(&self) -> &'static str {
        "advanced"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse {
        ProviderMetadataResponse {
            type_name: self.type_name().to_string(),
            server_capabilities: ServerCapabilities {
                plan_destroy: true,
                get_provider_schema_optional: false,
                move_resource_state: true,
            },
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

    async fn schema(
        &self,
        _ctx: Context,
        _request: ProviderSchemaRequest,
    ) -> ProviderSchemaResponse {
        let schema = SchemaBuilder::new()
            .attribute(
                AttributeBuilder::new("endpoint", AttributeType::String)
                    .required()
                    .description("API endpoint URL")
                    .validator(StringLengthValidator::min(5))
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("api_key", AttributeType::String)
                    .required()
                    .description("API authentication key")
                    .sensitive()
                    .validator(StringLengthValidator::between(32, 32))
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("environment", AttributeType::String)
                    .optional()
                    .description("Deployment environment")
                    .default(StaticDefault::string("production"))
                    .validator(StringOneOfValidator::create(vec![
                        "development".to_string(),
                        "staging".to_string(),
                        "production".to_string(),
                    ]))
                    .build(),
            )
            .build();

        ProviderSchemaResponse {
            schema,
            diagnostics: vec![],
        }
    }

    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureProviderRequest,
    ) -> ConfigureProviderResponse {
        let mut diagnostics = vec![];

        // Extract configuration
        let endpoint = match request.config.get_string(&AttributePath::new("endpoint")) {
            Ok(e) => e,
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Invalid endpoint",
                    format!("Failed to get endpoint: {}", e),
                ));
                return ConfigureProviderResponse {
                    diagnostics,
                    provider_data: None,
                };
            }
        };

        let api_key = match request.config.get_string(&AttributePath::new("api_key")) {
            Ok(k) => k,
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Invalid API key",
                    format!("Failed to get API key: {}", e),
                ));
                return ConfigureProviderResponse {
                    diagnostics,
                    provider_data: None,
                };
            }
        };

        let environment = request
            .config
            .get_string(&AttributePath::new("environment"))
            .unwrap_or_else(|_| "production".to_string());

        // Additional validation
        if environment == "production" && !endpoint.starts_with("https://") {
            diagnostics.push(Diagnostic::warning(
                "Insecure endpoint",
                "Production environment should use HTTPS",
            ));
        }

        // Create and store the client
        let config = ProviderConfig {
            api_endpoint: endpoint,
            api_key,
            environment,
        };
        let client = ApiClient::new(config);
        *self.client.write().await = Some(client.clone());

        ConfigureProviderResponse {
            diagnostics,
            provider_data: Some(Arc::new(client) as Arc<dyn Any + Send + Sync>),
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        request: ValidateProviderConfigRequest,
    ) -> ValidateProviderConfigResponse {
        let mut diagnostics = vec![];

        // Custom validation beyond schema validation
        if let Ok(endpoint) = request.config.get_string(&AttributePath::new("endpoint")) {
            if endpoint.contains("localhost") {
                if let Ok(env) = request
                    .config
                    .get_string(&AttributePath::new("environment"))
                {
                    if env == "production" {
                        diagnostics.push(Diagnostic::error(
                            "Invalid configuration",
                            "Cannot use localhost endpoint in production environment",
                        ));
                    }
                }
            }
        }

        ValidateProviderConfigResponse { diagnostics }
    }

    async fn stop(&self, _ctx: Context, _request: StopProviderRequest) -> StopProviderResponse {
        // Clean up any resources
        *self.client.write().await = None;
        StopProviderResponse { error: None }
    }

    fn resources(&self) -> HashMap<String, ResourceFactory> {
        let mut resources = HashMap::new();

        resources.insert(
            "advanced_server".to_string(),
            Box::new(|| Box::new(ServerResource::new()) as Box<dyn ResourceWithConfigure>)
                as ResourceFactory,
        );

        resources
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
        let mut data_sources = HashMap::new();

        data_sources.insert(
            "advanced_zone".to_string(),
            Box::new(|| Box::new(ZoneDataSource::new()) as Box<dyn DataSourceWithConfigure>)
                as DataSourceFactory,
        );

        data_sources
    }
}

// Server resource with advanced features
struct ServerResource {
    client: Option<ApiClient>,
}

impl ServerResource {
    fn new() -> Self {
        Self { client: None }
    }
}

#[async_trait::async_trait]
impl Resource for ServerResource {
    fn type_name(&self) -> &'static str {
        "advanced_server"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: self.type_name().to_string(),
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        _request: ValidateResourceConfigRequest,
    ) -> ValidateResourceConfigResponse {
        ValidateResourceConfigResponse {
            diagnostics: vec![],
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .attribute(
                AttributeBuilder::new("name", AttributeType::String)
                    .required()
                    .description("Server name")
                    .validator(StringLengthValidator::between(3, 63))
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("size", AttributeType::String)
                    .required()
                    .description("Server size")
                    .validator(StringOneOfValidator::create(vec![
                        "small".to_string(),
                        "medium".to_string(),
                        "large".to_string(),
                    ]))
                    .plan_modifier(RequiresReplace::create())
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("id", AttributeType::String)
                    .computed()
                    .description("Server ID")
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("status", AttributeType::String)
                    .computed()
                    .description("Server status")
                    .plan_modifier(UseStateForUnknown::create())
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ip_address", AttributeType::String)
                    .computed()
                    .description("Server IP address")
                    .plan_modifier(UseStateForUnknown::create())
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("tags", AttributeType::Map(Box::new(AttributeType::String)))
                    .optional()
                    .description("Server tags")
                    .build(),
            )
            .build();

        ResourceSchemaResponse {
            schema,
            diagnostics: vec![],
        }
    }

    async fn create(
        &self,
        _ctx: Context,
        request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        let mut diagnostics = vec![];

        let client = match &self.client {
            Some(c) => c,
            None => {
                diagnostics.push(Diagnostic::error("Provider not configured", ""));
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                };
            }
        };

        let name = match request.config.get_string(&AttributePath::new("name")) {
            Ok(n) => n,
            Err(e) => {
                diagnostics.push(Diagnostic::error("Invalid name", e.to_string()));
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                };
            }
        };

        let size = match request.config.get_string(&AttributePath::new("size")) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(Diagnostic::error("Invalid size", e.to_string()));
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                };
            }
        };

        // Create the server
        match client.create_server(&name, &size).await {
            Ok(id) => {
                let mut state = request.planned_state.clone();
                state
                    .set_string(&AttributePath::new("id"), id.clone())
                    .unwrap();
                state
                    .set_string(&AttributePath::new("status"), "creating".to_string())
                    .unwrap();

                CreateResourceResponse {
                    new_state: state,
                    private: request.planned_private,
                    diagnostics,
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic::error("Failed to create server", e));
                CreateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                }
            }
        }
    }

    async fn read(&self, _ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
        let mut diagnostics = vec![];

        let client = match &self.client {
            Some(c) => c,
            None => {
                return ReadResourceResponse {
                    new_state: None,
                    private: request.private,
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        let id = match request.current_state.get_string(&AttributePath::new("id")) {
            Ok(id) => id,
            Err(_) => {
                return ReadResourceResponse {
                    new_state: None,
                    private: request.private,
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        match client.get_server(&id).await {
            Ok(Some(info)) => {
                let mut state = request.current_state.clone();
                state
                    .set_string(&AttributePath::new("name"), info.name)
                    .unwrap();
                state
                    .set_string(&AttributePath::new("size"), info.size)
                    .unwrap();
                state
                    .set_string(&AttributePath::new("status"), info.status)
                    .unwrap();
                state
                    .set_string(&AttributePath::new("ip_address"), info.ip_address)
                    .unwrap();

                ReadResourceResponse {
                    new_state: Some(state),
                    private: request.private,
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                }
            }
            Ok(None) => ReadResourceResponse {
                new_state: None,
                private: request.private,
                diagnostics,
                deferred: None,
                new_identity: None,
            },
            Err(e) => {
                diagnostics.push(Diagnostic::error("Failed to read server", e));
                ReadResourceResponse {
                    new_state: Some(request.current_state),
                    private: request.private,
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                }
            }
        }
    }

    async fn update(
        &self,
        _ctx: Context,
        request: UpdateResourceRequest,
    ) -> UpdateResourceResponse {
        let mut diagnostics = vec![];

        let client = match &self.client {
            Some(c) => c,
            None => {
                diagnostics.push(Diagnostic::error("Provider not configured", ""));
                return UpdateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        let id = match request.prior_state.get_string(&AttributePath::new("id")) {
            Ok(id) => id,
            Err(e) => {
                diagnostics.push(Diagnostic::error("Invalid ID", e.to_string()));
                return UpdateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        let name = match request.config.get_string(&AttributePath::new("name")) {
            Ok(n) => n,
            Err(e) => {
                diagnostics.push(Diagnostic::error("Invalid name", e.to_string()));
                return UpdateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        let size = match request.config.get_string(&AttributePath::new("size")) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(Diagnostic::error("Invalid size", e.to_string()));
                return UpdateResourceResponse {
                    new_state: request.planned_state,
                    private: request.planned_private,
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        match client.update_server(&id, &name, &size).await {
            Ok(()) => UpdateResourceResponse {
                new_state: request.planned_state,
                private: request.planned_private,
                diagnostics,
                new_identity: None,
            },
            Err(e) => {
                diagnostics.push(Diagnostic::error("Failed to update server", e));
                UpdateResourceResponse {
                    new_state: request.prior_state,
                    private: request.planned_private,
                    diagnostics,
                    new_identity: None,
                }
            }
        }
    }

    async fn delete(
        &self,
        _ctx: Context,
        request: DeleteResourceRequest,
    ) -> DeleteResourceResponse {
        let mut diagnostics = vec![];

        let client = match &self.client {
            Some(c) => c,
            None => {
                return DeleteResourceResponse { diagnostics };
            }
        };

        let id = match request.prior_state.get_string(&AttributePath::new("id")) {
            Ok(id) => id,
            Err(_) => {
                return DeleteResourceResponse { diagnostics };
            }
        };

        if let Err(e) = client.delete_server(&id).await {
            diagnostics.push(Diagnostic::error("Failed to delete server", e));
        }

        DeleteResourceResponse { diagnostics }
    }
}

#[async_trait::async_trait]
impl ResourceWithConfigure for ServerResource {
    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureResourceRequest,
    ) -> ConfigureResourceResponse {
        if let Some(data) = request.provider_data {
            if let Ok(client) = data.downcast::<ApiClient>() {
                self.client = Some((*client).clone());
            }
        }
        ConfigureResourceResponse {
            diagnostics: vec![],
        }
    }
}

#[async_trait::async_trait]
impl ResourceWithImportState for ServerResource {
    async fn import_state(
        &self,
        ctx: Context,
        request: ImportResourceStateRequest,
    ) -> ImportResourceStateResponse {
        let mut response = ImportResourceStateResponse {
            imported_resources: vec![],
            diagnostics: vec![],
            deferred: None,
        };

        import_state_passthrough_id(&ctx, AttributePath::new("id"), &request, &mut response);

        response
    }
}

#[async_trait::async_trait]
impl ResourceWithModifyPlan for ServerResource {
    async fn modify_plan(&self, _ctx: Context, request: ModifyPlanRequest) -> ModifyPlanResponse {
        let mut diagnostics = vec![];
        let mut requires_replace = vec![];

        // Custom logic: changing to "large" size from "small" requires replace
        if let (Ok(current_size), Ok(planned_size)) = (
            request.prior_state.get_string(&AttributePath::new("size")),
            request
                .proposed_new_state
                .get_string(&AttributePath::new("size")),
        ) {
            if current_size == "small" && planned_size == "large" {
                requires_replace.push(AttributePath::new("size"));
                diagnostics.push(Diagnostic::warning(
                    "Server replacement required",
                    "Upgrading directly from small to large requires server replacement",
                ));
            }
        }

        ModifyPlanResponse {
            planned_state: request.proposed_new_state,
            requires_replace,
            planned_private: request.prior_private,
            diagnostics,
        }
    }
}

#[async_trait::async_trait]
impl ResourceWithUpgradeState for ServerResource {
    async fn upgrade_state(
        &self,
        _ctx: Context,
        request: UpgradeResourceStateRequest,
    ) -> UpgradeResourceStateResponse {
        let diagnostics = vec![];

        // Example: upgrade from version 0 to version 1
        // Version 0 had "instance_size", version 1 has "size"
        if request.version == 0 {
            // For this example, we'll create a new upgraded state
            let mut upgraded = DynamicValue::new(Dynamic::Map(HashMap::new()));

            // In a real implementation, you would parse request.raw_state and migrate fields
            // For this example, we'll just show the pattern with expected fields
            upgraded
                .set_string(&AttributePath::new("size"), "medium".to_string())
                .unwrap();
            upgraded
                .set_string(&AttributePath::new("id"), "srv-12345".to_string())
                .unwrap();
            upgraded
                .set_string(&AttributePath::new("name"), "upgraded-server".to_string())
                .unwrap();
            upgraded
                .set_string(&AttributePath::new("status"), "running".to_string())
                .unwrap();
            upgraded
                .set_string(&AttributePath::new("ip_address"), "10.0.0.1".to_string())
                .unwrap();

            return UpgradeResourceStateResponse {
                upgraded_state: upgraded,
                diagnostics,
            };
        }

        // For any other version, return as-is
        // In a real implementation, you would parse the raw state
        UpgradeResourceStateResponse {
            upgraded_state: DynamicValue::unknown(),
            diagnostics,
        }
    }
}

// Zone data source
struct ZoneDataSource {
    client: Option<ApiClient>,
}

impl ZoneDataSource {
    fn new() -> Self {
        Self { client: None }
    }
}

#[async_trait::async_trait]
impl DataSource for ZoneDataSource {
    fn type_name(&self) -> &'static str {
        "advanced_zone"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: DataSourceMetadataRequest,
    ) -> DataSourceMetadataResponse {
        DataSourceMetadataResponse {
            type_name: self.type_name().to_string(),
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        _request: ValidateDataSourceConfigRequest,
    ) -> ValidateDataSourceConfigResponse {
        ValidateDataSourceConfigResponse {
            diagnostics: vec![],
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: DataSourceSchemaRequest,
    ) -> DataSourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .attribute(
                AttributeBuilder::new("zone", AttributeType::String)
                    .required()
                    .description("Zone name")
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("region", AttributeType::String)
                    .computed()
                    .description("Region containing the zone")
                    .build(),
            )
            .attribute(
                AttributeBuilder::new(
                    "available_sizes",
                    AttributeType::List(Box::new(AttributeType::String)),
                )
                .computed()
                .description("Available server sizes in this zone")
                .build(),
            )
            .build();

        DataSourceSchemaResponse {
            schema,
            diagnostics: vec![],
        }
    }

    async fn read(&self, _ctx: Context, request: ReadDataSourceRequest) -> ReadDataSourceResponse {
        let mut diagnostics = vec![];

        let client = match &self.client {
            Some(c) => c,
            None => {
                diagnostics.push(Diagnostic::error("Provider not configured", ""));
                return ReadDataSourceResponse {
                    state: DynamicValue::unknown(),
                    deferred: None,
                    diagnostics,
                };
            }
        };

        let zone = match request.config.get_string(&AttributePath::new("zone")) {
            Ok(z) => z,
            Err(e) => {
                diagnostics.push(Diagnostic::error("Invalid zone", e.to_string()));
                return ReadDataSourceResponse {
                    state: DynamicValue::unknown(),
                    deferred: None,
                    diagnostics,
                };
            }
        };

        match client.get_zone_info(&zone).await {
            Ok(info) => {
                let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));
                state
                    .set_string(&AttributePath::new("zone"), info.zone)
                    .unwrap();
                state
                    .set_string(&AttributePath::new("region"), info.region)
                    .unwrap();

                let sizes: Vec<Dynamic> = info
                    .available_sizes
                    .iter()
                    .map(|s| Dynamic::String(s.to_string()))
                    .collect();
                state
                    .set_list(&AttributePath::new("available_sizes"), sizes)
                    .unwrap();

                ReadDataSourceResponse {
                    state,
                    deferred: None,
                    diagnostics,
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic::error("Failed to read zone info", e));
                ReadDataSourceResponse {
                    state: DynamicValue::unknown(),
                    deferred: None,
                    diagnostics,
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl DataSourceWithConfigure for ZoneDataSource {
    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureDataSourceRequest,
    ) -> ConfigureDataSourceResponse {
        if let Some(data) = request.provider_data {
            if let Ok(client) = data.downcast::<ApiClient>() {
                self.client = Some((*client).clone());
            }
        }
        ConfigureDataSourceResponse {
            diagnostics: vec![],
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Advanced Provider Example");
    println!("========================");

    // Create and configure provider
    let mut provider = AdvancedProvider::new();

    let mut config = DynamicValue::new(Dynamic::Map(HashMap::new()));
    config
        .set_string(
            &AttributePath::new("endpoint"),
            "https://api.example.com".to_string(),
        )
        .unwrap();
    config
        .set_string(&AttributePath::new("api_key"), "a".repeat(32))
        .unwrap();
    config
        .set_string(&AttributePath::new("environment"), "staging".to_string())
        .unwrap();

    let ctx = Context::new();

    // Validate configuration
    println!("\n1. Validating provider configuration...");
    let validate_response = provider
        .validate(
            ctx.clone(),
            ValidateProviderConfigRequest {
                config: config.clone(),
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    if !validate_response.diagnostics.is_empty() {
        println!("Validation diagnostics:");
        for diag in &validate_response.diagnostics {
            println!("  - {:?}: {}", diag.severity, diag.summary);
        }
    }

    // Configure provider
    println!("\n2. Configuring provider...");
    let configure_response = provider
        .configure(
            ctx.clone(),
            ConfigureProviderRequest {
                config,
                terraform_version: "1.0.0".to_string(),
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    if !configure_response.diagnostics.is_empty() {
        println!("Configuration failed!");
        return;
    }

    // Get provider metadata
    println!("\n3. Getting provider metadata...");
    let metadata_response = provider
        .metadata(ctx.clone(), ProviderMetadataRequest)
        .await;
    println!("Provider type: {}", metadata_response.type_name);
    println!(
        "Capabilities: plan_destroy={}, move_resource_state={}",
        metadata_response.server_capabilities.plan_destroy,
        metadata_response.server_capabilities.move_resource_state
    );

    // Create and configure a resource
    println!("\n4. Creating server resource...");
    let resources = provider.resources();
    let resource_factory = resources.get("advanced_server").unwrap();
    let mut resource = resource_factory();

    // Configure the resource with provider data
    let client = provider.client.read().await.clone().unwrap();
    resource
        .configure(
            ctx.clone(),
            ConfigureResourceRequest {
                provider_data: Some(Arc::new(client.clone()) as Arc<dyn Any + Send + Sync>),
            },
        )
        .await;

    // Get resource schema
    let schema_response = resource.schema(ctx.clone(), ResourceSchemaRequest).await;
    println!(
        "Resource has {} attributes",
        schema_response.schema.block.attributes.len()
    );

    // Create a server
    println!("\n5. Creating a server...");
    let mut server_config = DynamicValue::new(Dynamic::Map(HashMap::new()));
    server_config
        .set_string(&AttributePath::new("name"), "test-server".to_string())
        .unwrap();
    server_config
        .set_string(&AttributePath::new("size"), "medium".to_string())
        .unwrap();

    let mut tags = HashMap::new();
    tags.insert("env".to_string(), Dynamic::String("test".to_string()));
    tags.insert("team".to_string(), Dynamic::String("platform".to_string()));
    server_config
        .set_map(&AttributePath::new("tags"), tags)
        .unwrap();

    let create_response = resource
        .create(
            ctx.clone(),
            CreateResourceRequest {
                type_name: "advanced_server".to_string(),
                config: server_config.clone(),
                planned_state: server_config.clone(),
                planned_private: Vec::new(),
                provider_meta: None,
            },
        )
        .await;

    if !create_response.diagnostics.is_empty() {
        println!("Create failed!");
        return;
    }

    let server_id = create_response
        .new_state
        .get_string(&AttributePath::new("id"))
        .unwrap();
    println!("Created server with ID: {}", server_id);

    // Test import functionality
    println!("\n6. Testing import functionality...");
    // In a real provider, you would test import on the actual ServerResource type
    // For this example, we'll create a new instance directly
    let server_resource = ServerResource::new();

    let import_response = server_resource
        .import_state(
            ctx.clone(),
            ImportResourceStateRequest {
                type_name: "advanced_server".to_string(),
                id: "srv-imported-123".to_string(),
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
                identity: None,
            },
        )
        .await;

    if !import_response.imported_resources.is_empty() {
        println!(
            "Successfully imported {} resources",
            import_response.imported_resources.len()
        );
    }

    // Test plan modification
    println!("\n7. Testing plan modification...");
    let mut new_state = create_response.new_state.clone();
    new_state
        .set_string(&AttributePath::new("size"), "large".to_string())
        .unwrap();

    let modify_response = server_resource
        .modify_plan(
            ctx.clone(),
            ModifyPlanRequest {
                type_name: "advanced_server".to_string(),
                config: server_config,
                prior_state: create_response.new_state.clone(),
                proposed_new_state: new_state,
                prior_private: Vec::new(),
                provider_meta: None,
            },
        )
        .await;

    if !modify_response.requires_replace.is_empty() {
        println!("Plan modification detected replacements required for:");
        for path in &modify_response.requires_replace {
            println!("  - {:?}", path);
        }
    }

    // Test data source
    println!("\n8. Testing data source...");
    let data_sources = provider.data_sources();
    let ds_factory = data_sources.get("advanced_zone").unwrap();
    let mut data_source = ds_factory();

    data_source
        .configure(
            ctx.clone(),
            ConfigureDataSourceRequest {
                provider_data: Some(Arc::new(client) as Arc<dyn Any + Send + Sync>),
            },
        )
        .await;

    let mut zone_config = DynamicValue::new(Dynamic::Map(HashMap::new()));
    zone_config
        .set_string(&AttributePath::new("zone"), "us-east-1a".to_string())
        .unwrap();

    let ds_response = data_source
        .read(
            ctx.clone(),
            ReadDataSourceRequest {
                type_name: "advanced_zone".to_string(),
                config: zone_config,
                provider_meta: None,
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    if ds_response.diagnostics.is_empty() {
        println!(
            "Zone region: {}",
            ds_response
                .state
                .get_string(&AttributePath::new("region"))
                .unwrap()
        );
        println!(
            "Available sizes: {:?}",
            ds_response
                .state
                .get_list(&AttributePath::new("available_sizes"))
                .unwrap()
        );
    }

    // Stop provider
    println!("\n9. Stopping provider...");
    provider.stop(ctx, StopProviderRequest).await;

    println!("\nAdvanced example completed!");
}
