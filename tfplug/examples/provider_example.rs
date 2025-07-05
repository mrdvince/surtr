//! Example showing how to use the new tfplug Provider API

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tfplug::context::Context;
use tfplug::provider::*;
use tfplug::resource::*;
use tfplug::schema::{AttributeType, Schema};
use tfplug::types::{
    AttributePath, AttributePathStep, ClientCapabilities, Diagnostic, DiagnosticSeverity, Dynamic,
    DynamicValue, ServerCapabilities,
};
use tfplug::{AttributeBuilder, SchemaBuilder};
use tokio::sync::RwLock;

// Example provider state that resources can access
#[derive(Clone)]
struct ApiClient {
    endpoint: String,
}

impl ApiClient {
    fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    async fn create_user(
        &self,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate async API call to endpoint
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("Creating user '{}' at {}", name, self.endpoint);
        Ok(format!("user-{}-{}", name, uuid::Uuid::new_v4()))
    }

    async fn update_user(
        &self,
        id: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Simulate async API call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("Updating user '{}' with name '{}'", id, name);
        Ok(())
    }

    async fn delete_user(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Simulate async API call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("Deleting user '{}'", id);
        Ok(())
    }

    async fn get_user(
        &self,
        id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate async API call
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        println!("Getting user '{}'", id);
        Ok(Some("active".to_string()))
    }
}

// Provider data that will be passed to resources
#[derive(Clone)]
struct ProviderData {
    client: ApiClient,
}

// Provider implementation
struct ExampleProvider {
    client: Arc<RwLock<Option<ApiClient>>>,
}

impl ExampleProvider {
    fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
        }
    }

    fn create_provider_schema(&self) -> Schema {
        SchemaBuilder::new()
            .attribute(
                AttributeBuilder::new("endpoint", AttributeType::String)
                    .required()
                    .description("API endpoint URL")
                    .build(),
            )
            .build()
    }
}

#[async_trait]
impl Provider for ExampleProvider {
    fn type_name(&self) -> &str {
        "example"
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
            schema: self.create_provider_schema(),
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

    async fn validate(
        &self,
        _ctx: Context,
        request: ValidateProviderConfigRequest,
    ) -> ValidateProviderConfigResponse {
        let mut diagnostics = vec![];

        // Validate endpoint is provided
        if let Dynamic::Map(map) = &request.config.value {
            if !map.contains_key("endpoint") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "endpoint is required".to_string(),
                    detail: String::new(),
                    attribute: None,
                });
            }
        }

        ValidateProviderConfigResponse { diagnostics }
    }

    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureProviderRequest,
    ) -> ConfigureProviderResponse {
        let mut diagnostics = vec![];

        let endpoint = if let Dynamic::Map(map) = &request.config.value {
            match map.get("endpoint") {
                Some(Dynamic::String(e)) => e.clone(),
                _ => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "endpoint must be a string".to_string(),
                        detail: String::new(),
                        attribute: None,
                    });
                    return ConfigureProviderResponse {
                        diagnostics,
                        provider_data: None,
                    };
                }
            }
        } else {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                summary: "configuration must be an object".to_string(),
                detail: String::new(),
                attribute: None,
            });
            return ConfigureProviderResponse {
                diagnostics,
                provider_data: None,
            };
        };

        let client = ApiClient::new(endpoint);
        *self.client.write().await = Some(client.clone());

        // Return provider data that will be passed to resources
        ConfigureProviderResponse {
            diagnostics,
            provider_data: Some(Arc::new(ProviderData { client }) as Arc<dyn Any + Send + Sync>),
        }
    }

    async fn stop(&self, _ctx: Context, _request: StopProviderRequest) -> StopProviderResponse {
        StopProviderResponse { error: None }
    }

    fn resources(&self) -> HashMap<String, ResourceFactory> {
        let mut resources = HashMap::new();

        // Factory pattern: return a closure that creates new resource instances
        resources.insert(
            "example_user".to_string(),
            Box::new(|| Box::new(UserResource::new()) as Box<dyn ResourceWithConfigure>)
                as ResourceFactory,
        );

        resources
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
        // No data sources in this example
        HashMap::new()
    }
}

// Resource implementation - now stateless with provider data passed in configure
struct UserResource {
    client: Option<ApiClient>,
}

impl UserResource {
    fn new() -> Self {
        Self { client: None }
    }

    fn create_schema(&self) -> Schema {
        SchemaBuilder::new()
            .attribute(
                AttributeBuilder::new("name", AttributeType::String)
                    .required()
                    .description("User name")
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("id", AttributeType::String)
                    .computed()
                    .description("User ID")
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("status", AttributeType::String)
                    .computed()
                    .description("User status")
                    .build(),
            )
            .build()
    }

    fn get_client(&self) -> Result<&ApiClient, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .as_ref()
            .ok_or_else(|| "Provider not configured".into())
    }
}

#[async_trait]
impl Resource for UserResource {
    fn type_name(&self) -> &str {
        "example_user"
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

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        ResourceSchemaResponse {
            schema: self.create_schema(),
            diagnostics: vec![],
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        request: ValidateResourceConfigRequest,
    ) -> ValidateResourceConfigResponse {
        let mut diagnostics = vec![];

        // Validate name is provided and is a string
        if let Dynamic::Map(map) = &request.config.value {
            match map.get("name") {
                None => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "name is required".to_string(),
                        detail: String::new(),
                        attribute: Some(AttributePath {
                            steps: vec![AttributePathStep::AttributeName("name".to_string())],
                        }),
                    });
                }
                Some(Dynamic::String(_)) => {}
                Some(_) => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "name must be a string".to_string(),
                        detail: String::new(),
                        attribute: Some(AttributePath {
                            steps: vec![AttributePathStep::AttributeName("name".to_string())],
                        }),
                    });
                }
            }
        }

        ValidateResourceConfigResponse { diagnostics }
    }

    async fn create(
        &self,
        _ctx: Context,
        request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        let mut diagnostics = vec![];

        let client = match self.get_client() {
            Ok(c) => c,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to get client".to_string(),
                    detail: e.to_string(),
                    attribute: None,
                });
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                };
            }
        };

        let name = if let Dynamic::Map(map) = &request.config.value {
            match map.get("name") {
                Some(Dynamic::String(n)) => n.clone(),
                _ => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "name is required and must be a string".to_string(),
                        detail: String::new(),
                        attribute: None,
                    });
                    return CreateResourceResponse {
                        new_state: request.planned_state,
                        private: vec![],
                        diagnostics,
                    };
                }
            }
        } else {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                summary: "configuration must be an object".to_string(),
                detail: String::new(),
                attribute: None,
            });
            return CreateResourceResponse {
                new_state: request.planned_state,
                private: vec![],
                diagnostics,
            };
        };

        match client.create_user(&name).await {
            Ok(id) => {
                // Build new state with all attributes
                let mut state_map = HashMap::new();
                state_map.insert("name".to_string(), Dynamic::String(name));
                state_map.insert("id".to_string(), Dynamic::String(id));
                state_map.insert("status".to_string(), Dynamic::String("active".to_string()));

                CreateResourceResponse {
                    new_state: DynamicValue::new(Dynamic::Map(state_map)),
                    private: vec![],
                    diagnostics,
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to create user".to_string(),
                    detail: e.to_string(),
                    attribute: None,
                });
                CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                }
            }
        }
    }

    async fn read(&self, _ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
        let mut diagnostics = vec![];

        let client = match self.get_client() {
            Ok(c) => c,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to get client".to_string(),
                    detail: e.to_string(),
                    attribute: None,
                });
                return ReadResourceResponse {
                    new_state: None,
                    private: vec![],
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        let id = if let Dynamic::Map(map) = &request.current_state.value {
            match map.get("id") {
                Some(Dynamic::String(id)) => id.clone(),
                _ => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "id is required in state".to_string(),
                        detail: String::new(),
                        attribute: None,
                    });
                    return ReadResourceResponse {
                        new_state: None,
                        private: vec![],
                        diagnostics,
                        deferred: None,
                        new_identity: None,
                    };
                }
            }
        } else {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                summary: "state must be an object".to_string(),
                detail: String::new(),
                attribute: None,
            });
            return ReadResourceResponse {
                new_state: None,
                private: vec![],
                diagnostics,
                deferred: None,
                new_identity: None,
            };
        };

        match client.get_user(&id).await {
            Ok(Some(status)) => {
                // Update state with new status
                if let Dynamic::Map(mut map) = request.current_state.value {
                    map.insert("status".to_string(), Dynamic::String(status));
                    ReadResourceResponse {
                        new_state: Some(DynamicValue::new(Dynamic::Map(map))),
                        private: vec![],
                        diagnostics,
                        deferred: None,
                        new_identity: None,
                    }
                } else {
                    ReadResourceResponse {
                        new_state: Some(request.current_state),
                        private: vec![],
                        diagnostics,
                        deferred: None,
                        new_identity: None,
                    }
                }
            }
            Ok(None) => {
                // User not found - return null to indicate resource doesn't exist
                ReadResourceResponse {
                    new_state: None,
                    private: vec![],
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to read user".to_string(),
                    detail: e.to_string(),
                    attribute: None,
                });
                ReadResourceResponse {
                    new_state: Some(request.current_state),
                    private: vec![],
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

        let client = match self.get_client() {
            Ok(c) => c,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to get client".to_string(),
                    detail: e.to_string(),
                    attribute: None,
                });
                return UpdateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        let (id, prior_name) = if let Dynamic::Map(map) = &request.prior_state.value {
            let id = match map.get("id") {
                Some(Dynamic::String(id)) => id.clone(),
                _ => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "id is required in state".to_string(),
                        detail: String::new(),
                        attribute: None,
                    });
                    return UpdateResourceResponse {
                        new_state: request.planned_state,
                        private: vec![],
                        diagnostics,
                        new_identity: None,
                    };
                }
            };
            let name = match map.get("name") {
                Some(Dynamic::String(n)) => n.clone(),
                _ => String::new(),
            };
            (id, name)
        } else {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                summary: "state must be an object".to_string(),
                detail: String::new(),
                attribute: None,
            });
            return UpdateResourceResponse {
                new_state: request.planned_state,
                private: vec![],
                diagnostics,
                new_identity: None,
            };
        };

        let new_name = if let Dynamic::Map(map) = &request.config.value {
            match map.get("name") {
                Some(Dynamic::String(n)) => n.clone(),
                _ => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "name is required and must be a string".to_string(),
                        detail: String::new(),
                        attribute: None,
                    });
                    return UpdateResourceResponse {
                        new_state: request.planned_state,
                        private: vec![],
                        diagnostics,
                        new_identity: None,
                    };
                }
            }
        } else {
            prior_name // Keep existing name if not provided
        };

        match client.update_user(&id, &new_name).await {
            Ok(()) => UpdateResourceResponse {
                new_state: request.planned_state,
                private: vec![],
                diagnostics,
                new_identity: None,
            },
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to update user".to_string(),
                    detail: e.to_string(),
                    attribute: None,
                });
                UpdateResourceResponse {
                    new_state: request.prior_state,
                    private: vec![],
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

        let client = match self.get_client() {
            Ok(c) => c,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to get client".to_string(),
                    detail: e.to_string(),
                    attribute: None,
                });
                return DeleteResourceResponse { diagnostics };
            }
        };

        let id = if let Dynamic::Map(map) = &request.prior_state.value {
            match map.get("id") {
                Some(Dynamic::String(id)) => id.clone(),
                _ => {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        summary: "id is required in state".to_string(),
                        detail: String::new(),
                        attribute: None,
                    });
                    return DeleteResourceResponse { diagnostics };
                }
            }
        } else {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                summary: "state must be an object".to_string(),
                detail: String::new(),
                attribute: None,
            });
            return DeleteResourceResponse { diagnostics };
        };

        if let Err(e) = client.delete_user(&id).await {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                summary: "Failed to delete user".to_string(),
                detail: e.to_string(),
                attribute: None,
            });
        }

        DeleteResourceResponse { diagnostics }
    }
}

// Implement ResourceWithConfigure to receive provider data
#[async_trait]
impl ResourceWithConfigure for UserResource {
    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureResourceRequest,
    ) -> ConfigureResourceResponse {
        let mut diagnostics = vec![];

        // Downcast provider data to our specific type
        if let Some(data) = request.provider_data {
            if let Ok(provider_data) = data.downcast::<ProviderData>() {
                self.client = Some(provider_data.client.clone());
            } else {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to downcast provider data".to_string(),
                    detail: "Expected ProviderData type".to_string(),
                    attribute: None,
                });
            }
        }

        ConfigureResourceResponse { diagnostics }
    }
}

#[tokio::main]
async fn main() {
    println!("Terraform Provider Example Using New tfplug API");

    // Create provider
    let mut provider = ExampleProvider::new();

    // Configure provider
    let mut config_map = HashMap::new();
    config_map.insert(
        "endpoint".to_string(),
        Dynamic::String("https://api.example.com".to_string()),
    );

    let ctx = Context::new();
    let configure_response = provider
        .configure(
            ctx.clone(),
            ConfigureProviderRequest {
                config: DynamicValue::new(Dynamic::Map(config_map)),
                terraform_version: "1.0.0".to_string(),
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    if !configure_response.diagnostics.is_empty() {
        eprintln!("Configuration failed: {:?}", configure_response.diagnostics);
        return;
    }

    println!("\nDemonstrating concurrent resource operations:");

    let mut handles = vec![];

    // Create multiple resources and configure them with provider data
    // We'll share the client instead of cloning provider_data
    let client = provider.client.read().await.clone().unwrap();

    for i in 0..3 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            // Create new resource instance by calling the factory directly
            let mut resource = Box::new(UserResource::new()) as Box<dyn ResourceWithConfigure>;

            // Configure resource with provider data
            let configure_res = resource
                .configure(
                    Context::new(),
                    ConfigureResourceRequest {
                        provider_data: Some(Arc::new(ProviderData {
                            client: client.clone(),
                        })
                            as Arc<dyn Any + Send + Sync>),
                    },
                )
                .await;

            if !configure_res.diagnostics.is_empty() {
                eprintln!(
                    "Failed to configure resource: {:?}",
                    configure_res.diagnostics
                );
                return None;
            }

            // Create user
            let mut user_config = HashMap::new();
            user_config.insert("name".to_string(), Dynamic::String(format!("user-{}", i)));

            let create_response = resource
                .create(
                    Context::new(),
                    CreateResourceRequest {
                        type_name: "example_user".to_string(),
                        config: DynamicValue::new(Dynamic::Map(user_config.clone())),
                        planned_state: DynamicValue::new(Dynamic::Map(user_config)),
                        planned_private: vec![],
                        provider_meta: None,
                    },
                )
                .await;

            if create_response.diagnostics.is_empty() {
                println!("User {} created successfully!", i);
                if let Dynamic::Map(state) = &create_response.new_state.value {
                    if let Some(Dynamic::String(id)) = state.get("id") {
                        println!("  ID: {}", id);
                    }
                }
                Some((resource, create_response.new_state))
            } else {
                eprintln!(
                    "Failed to create user {}: {:?}",
                    i, create_response.diagnostics
                );
                None
            }
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .filter_map(|r| r)
        .collect();

    println!("\nCreated {} users concurrently", results.len());

    // Demonstrate concurrent reads
    println!("\nReading all users concurrently:");
    let mut read_handles = vec![];

    for (resource, state) in results {
        let handle = tokio::spawn(async move {
            let read_response = resource
                .read(
                    Context::new(),
                    ReadResourceRequest {
                        type_name: "example_user".to_string(),
                        current_state: state.clone(),
                        private: vec![],
                        provider_meta: None,
                        client_capabilities: ClientCapabilities {
                            deferral_allowed: false,
                            write_only_attributes_allowed: false,
                        },
                        current_identity: None,
                    },
                )
                .await;

            if !read_response.diagnostics.is_empty() {
                eprintln!("Read failed: {:?}", read_response.diagnostics);
                return;
            }

            if let Some(new_state) = &read_response.new_state {
                if let Dynamic::Map(state_map) = &new_state.value {
                    let id = state_map.get("id").and_then(|v| {
                        if let Dynamic::String(s) = v {
                            Some(s)
                        } else {
                            None
                        }
                    });
                    let status = state_map.get("status").and_then(|v| {
                        if let Dynamic::String(s) = v {
                            Some(s)
                        } else {
                            None
                        }
                    });

                    println!(
                        "  User ID: {}, Status: {}",
                        id.unwrap_or(&"unknown".to_string()),
                        status.unwrap_or(&"unknown".to_string())
                    );
                }
            }
        });
        read_handles.push(handle);
    }

    futures::future::join_all(read_handles).await;
}
