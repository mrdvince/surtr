//! Example showing how to use the ProviderV2 async architecture

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tfplug::provider::{DataSourceSchema, ResourceSchema};
use tfplug::request::{ResourceSchemaResponse, *};
use tfplug::types::{Config, Diagnostics, Dynamic, State};
use tfplug::{AttributeBuilder, Result, SchemaBuilder, StateBuilder};
use tfplug::{DataSourceV2, ProviderV2, ResourceV2};
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

    async fn create_user(&self, name: &str) -> Result<String> {
        // Simulate async API call to endpoint
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("Creating user '{}' at {}", name, self.endpoint);
        Ok(format!("user-{}-{}", name, uuid::Uuid::new_v4()))
    }

    async fn update_user(&self, id: &str, name: &str) -> Result<()> {
        // Simulate async API call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("Updating user '{}' with name '{}'", id, name);
        Ok(())
    }

    async fn delete_user(&self, id: &str) -> Result<()> {
        // Simulate async API call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("Deleting user '{}'", id);
        Ok(())
    }

    async fn get_user(&self, id: &str) -> Result<Option<String>> {
        // Simulate async API call
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        println!("Getting user '{}'", id);
        Ok(Some("active".to_string()))
    }
}

// Provider implementation
struct ExampleProvider {
    client: Arc<RwLock<Option<ApiClient>>>,
    resource_schemas: HashMap<String, ResourceSchema>,
    data_source_schemas: HashMap<String, DataSourceSchema>,
}

impl ExampleProvider {
    fn new() -> Self {
        let mut resource_schemas = HashMap::new();
        resource_schemas.insert(
            "example_user".to_string(),
            SchemaBuilder::new()
                .attribute(
                    "name",
                    AttributeBuilder::string("name")
                        .required()
                        .description("User name"),
                )
                .attribute(
                    "id",
                    AttributeBuilder::string("id")
                        .computed()
                        .description("User ID"),
                )
                .attribute(
                    "status",
                    AttributeBuilder::string("status")
                        .computed()
                        .description("User status"),
                )
                .build_resource(1),
        );

        Self {
            client: Arc::new(RwLock::new(None)),
            resource_schemas,
            data_source_schemas: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProviderV2 for ExampleProvider {
    async fn configure(&mut self, request: ConfigureRequest) -> ConfigureResponse {
        let mut diagnostics = Diagnostics::new();

        let endpoint = match request.config.get_string("endpoint") {
            Some(e) => e,
            None => {
                diagnostics.add_error("endpoint is required", None::<String>);
                return ConfigureResponse { diagnostics };
            }
        };

        let mut client_lock = self.client.write().await;
        *client_lock = Some(ApiClient::new(endpoint));

        ConfigureResponse { diagnostics }
    }

    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
        match name {
            "example_user" => Ok(Box::new(UserResource::new(self.client.clone()))),
            _ => Err(format!("Unknown resource: {}", name).into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>> {
        Err(format!("Unknown data source: {}", name).into())
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        self.resource_schemas.clone()
    }

    async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema> {
        self.data_source_schemas.clone()
    }
}

// Stateless resource implementation
struct UserResource {
    client: Arc<RwLock<Option<ApiClient>>>,
}

impl UserResource {
    fn new(client: Arc<RwLock<Option<ApiClient>>>) -> Self {
        Self { client }
    }

    async fn get_client(&self) -> Result<ApiClient> {
        self.client
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| "Provider not configured".into())
    }
}

#[async_trait]
impl ResourceV2 for UserResource {
    async fn schema(&self, _: SchemaRequest) -> ResourceSchemaResponse {
        let schema = ResourceSchema {
            version: 1,
            attributes: HashMap::new(), // Would normally populate this
        };

        ResourceSchemaResponse {
            schema,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create(&self, request: CreateRequest) -> CreateResponse {
        let mut diagnostics = Diagnostics::new();

        let client = match self.get_client().await {
            Ok(c) => c,
            Err(e) => {
                diagnostics.add_error("Failed to get client", Some(&e.to_string()));
                return CreateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }
        };

        let name = match request.config.require_string("name") {
            Ok(n) => n,
            Err(e) => {
                diagnostics.add_error("name is required", Some(&e.to_string()));
                return CreateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }
        };

        match client.create_user(&name).await {
            Ok(id) => {
                let state = StateBuilder::from_config(&request.config)
                    .string("id", &id)
                    .string("status", "active")
                    .build();

                CreateResponse { state, diagnostics }
            }
            Err(e) => {
                diagnostics.add_error("Failed to create user", Some(&e.to_string()));
                CreateResponse {
                    state: request.planned_state,
                    diagnostics,
                }
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> ReadResponse {
        let mut diagnostics = Diagnostics::new();

        let client = match self.get_client().await {
            Ok(c) => c,
            Err(e) => {
                diagnostics.add_error("Failed to get client", Some(&e.to_string()));
                return ReadResponse {
                    state: None,
                    diagnostics,
                };
            }
        };

        let id = match request.current_state.require_string("id") {
            Ok(id) => id,
            Err(e) => {
                diagnostics.add_error("id is required in state", Some(&e.to_string()));
                return ReadResponse {
                    state: None,
                    diagnostics,
                };
            }
        };

        match client.get_user(&id).await {
            Ok(Some(status)) => {
                let mut state = request.current_state.clone();
                state
                    .values
                    .insert("status".to_string(), Dynamic::String(status));

                ReadResponse {
                    state: Some(state),
                    diagnostics,
                }
            }
            Ok(None) => {
                // User not found
                ReadResponse {
                    state: None,
                    diagnostics,
                }
            }
            Err(e) => {
                diagnostics.add_error("Failed to read user", Some(&e.to_string()));
                ReadResponse {
                    state: Some(request.current_state),
                    diagnostics,
                }
            }
        }
    }

    async fn update(&self, request: UpdateRequest) -> UpdateResponse {
        let mut diagnostics = Diagnostics::new();

        let client = match self.get_client().await {
            Ok(c) => c,
            Err(e) => {
                diagnostics.add_error("Failed to get client", Some(&e.to_string()));
                return UpdateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }
        };

        let id = match request.current_state.require_string("id") {
            Ok(id) => id,
            Err(e) => {
                diagnostics.add_error("id is required in state", Some(&e.to_string()));
                return UpdateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }
        };

        let name = match request.config.require_string("name") {
            Ok(n) => n,
            Err(e) => {
                diagnostics.add_error("name is required", Some(&e.to_string()));
                return UpdateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }
        };

        match client.update_user(&id, &name).await {
            Ok(()) => UpdateResponse {
                state: request.planned_state,
                diagnostics,
            },
            Err(e) => {
                diagnostics.add_error("Failed to update user", Some(&e.to_string()));
                UpdateResponse {
                    state: request.current_state,
                    diagnostics,
                }
            }
        }
    }

    async fn delete(&self, request: DeleteRequest) -> DeleteResponse {
        let mut diagnostics = Diagnostics::new();

        let client = match self.get_client().await {
            Ok(c) => c,
            Err(e) => {
                diagnostics.add_error("Failed to get client", Some(&e.to_string()));
                return DeleteResponse { diagnostics };
            }
        };

        let id = match request.current_state.require_string("id") {
            Ok(id) => id,
            Err(e) => {
                diagnostics.add_error("id is required in state", Some(&e.to_string()));
                return DeleteResponse { diagnostics };
            }
        };

        if let Err(e) = client.delete_user(&id).await {
            diagnostics.add_error("Failed to delete user", Some(&e.to_string()));
        }

        DeleteResponse { diagnostics }
    }
}

#[tokio::main]
async fn main() {
    println!("Provider V2 Async Example");

    // Create provider
    let mut provider = ExampleProvider::new();

    // Configure it
    let mut config = Config::new();
    config.values.insert(
        "endpoint".to_string(),
        Dynamic::String("https://api.example.com".to_string()),
    );

    let configure_response = provider
        .configure(ConfigureRequest {
            context: tfplug::context::Context::new(),
            config,
        })
        .await;

    if !configure_response.diagnostics.errors.is_empty() {
        eprintln!(
            "Configuration failed: {:?}",
            configure_response.diagnostics.errors
        );
        return;
    }

    // Create a resource
    let resource = provider.create_resource("example_user").await.unwrap();

    // Demonstrate concurrent operations
    println!("\nDemonstrating concurrent resource operations:");

    let resource_arc = Arc::new(resource);
    let mut handles = vec![];

    // Create multiple users concurrently
    for i in 0..3 {
        let resource_clone = resource_arc.clone();
        let handle = tokio::spawn(async move {
            let mut user_config = Config::new();
            user_config
                .values
                .insert("name".to_string(), Dynamic::String(format!("user-{}", i)));

            let create_response = resource_clone
                .create(CreateRequest {
                    context: tfplug::context::Context::new(),
                    config: user_config,
                    planned_state: State::new(),
                })
                .await;

            if create_response.diagnostics.errors.is_empty() {
                println!("User {} created successfully!", i);
                println!("  ID: {:?}", create_response.state.get_string("id"));
                create_response.state
            } else {
                eprintln!(
                    "Failed to create user {}: {:?}",
                    i, create_response.diagnostics.errors
                );
                State::new()
            }
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let states: Vec<State> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    println!("\nCreated {} users concurrently", states.len());

    // Demonstrate concurrent reads
    println!("\nReading all users concurrently:");
    let mut read_handles = vec![];

    for state in states {
        let resource_clone = resource_arc.clone();
        let handle = tokio::spawn(async move {
            let read_response = resource_clone
                .read(ReadRequest {
                    context: tfplug::context::Context::new(),
                    current_state: state.clone(),
                })
                .await;

            if let Some(read_state) = read_response.state {
                println!(
                    "  User ID: {:?}, Status: {:?}",
                    read_state.get_string("id"),
                    read_state.get_string("status")
                );
            }
        });
        read_handles.push(handle);
    }

    futures::future::join_all(read_handles).await;
}
