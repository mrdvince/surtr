//! Advanced example showing V2 async architecture patterns

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tfplug::attribute_type::AttributeType;
use tfplug::provider::{DataSourceSchema, ResourceSchema};
use tfplug::request::{DataSourceSchemaResponse, ResourceSchemaResponse, *};
use tfplug::types::{Config, Diagnostics, Dynamic, State};
use tfplug::{AttributeBuilder, Result, SchemaBuilder, StateBuilder};
use tfplug::{DataSourceV2, ProviderV2, ResourceV2};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{timeout, Duration};

// Shared provider configuration with rate limiting and timeout control
#[derive(Clone)]
struct ProviderConfig {
    api_endpoint: String,
    api_key: String,
    max_concurrent_operations: usize,
    operation_timeout: Duration,
}

// API client with built-in rate limiting and retry logic
#[derive(Clone)]
struct ApiClient {
    config: ProviderConfig,
    semaphore: Arc<Semaphore>,
}

impl ApiClient {
    fn new(config: ProviderConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_operations));
        Self { config, semaphore }
    }

    async fn execute_with_retry<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Acquire rate limiting permit
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

        // Execute with timeout and retry
        let mut retries = 3;
        loop {
            match timeout(self.config.operation_timeout, operation()).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) if retries > 0 => {
                    retries -= 1;
                    println!(
                        "Operation failed, retrying... ({} retries left): {}",
                        retries, e
                    );
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    if retries > 0 {
                        retries -= 1;
                        println!(
                            "Operation timed out, retrying... ({} retries left)",
                            retries
                        );
                    } else {
                        return Err("Operation timed out after all retries".into());
                    }
                }
            }
        }
    }

    async fn create_resource(&self, resource_type: &str, name: &str) -> Result<String> {
        self.execute_with_retry(|| async {
            // Use api_endpoint and api_key for realistic API simulation
            let url = format!("{}/api/v1/{}", self.config.api_endpoint, resource_type);
            println!("Creating {} '{}' at endpoint: {}", resource_type, name, url);
            println!("Using API key: {}...", &self.config.api_key[..8]);

            // Simulate API call
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(format!(
                "{}-{}-{}",
                resource_type,
                name,
                uuid::Uuid::new_v4()
            ))
        })
        .await
    }

    async fn get_resource(&self, id: &str) -> Result<Option<HashMap<String, String>>> {
        self.execute_with_retry(|| async {
            // Use api_endpoint and api_key for realistic API simulation
            let url = format!("{}/api/v1/resources/{}", self.config.api_endpoint, id);
            println!("Reading resource '{}' from endpoint: {}", id, url);
            println!(
                "Authenticating with API key: {}...",
                &self.config.api_key[..8]
            );

            // Simulate API call
            tokio::time::sleep(Duration::from_millis(30)).await;
            let mut data = HashMap::new();
            data.insert("id".to_string(), id.to_string());
            data.insert("status".to_string(), "active".to_string());
            Ok(Some(data))
        })
        .await
    }

    async fn delete_resource(&self, id: &str) -> Result<()> {
        self.execute_with_retry(|| async {
            // Use api_endpoint and api_key for realistic API simulation
            let url = format!("{}/api/v1/resources/{}", self.config.api_endpoint, id);
            println!("Deleting resource '{}' at endpoint: {}", id, url);
            println!(
                "Authenticating with API key: {}...",
                &self.config.api_key[..8]
            );

            // Simulate API call
            tokio::time::sleep(Duration::from_millis(40)).await;
            println!("Successfully deleted resource: {}", id);
            Ok(())
        })
        .await
    }

    async fn list_servers(&self, filter: Option<&str>) -> Result<Vec<HashMap<String, String>>> {
        self.execute_with_retry(|| async {
            // Use api_endpoint and api_key for realistic API simulation
            let mut url = format!("{}/api/v1/servers", self.config.api_endpoint);
            if let Some(f) = filter {
                url = format!("{}?filter={}", url, f);
            }
            println!("Listing servers from endpoint: {}", url);
            println!(
                "Authenticating with API key: {}...",
                &self.config.api_key[..8]
            );

            // Simulate API call
            tokio::time::sleep(Duration::from_millis(60)).await;

            let mut servers = vec![];
            for i in 0..5 {
                let server_name = format!("Server {}", i);
                if let Some(f) = filter {
                    if !server_name.contains(f) {
                        continue;
                    }
                }

                let mut server = HashMap::new();
                server.insert("id".to_string(), format!("server-{}", i));
                server.insert("name".to_string(), server_name);
                server.insert("status".to_string(), "active".to_string());
                server.insert("cpu_count".to_string(), format!("{}", 2 + i));
                servers.push(server);
            }

            Ok(servers)
        })
        .await
    }
}

// Advanced provider with sophisticated configuration
struct AdvancedProvider {
    client: Arc<RwLock<Option<ApiClient>>>,
    resource_schemas: HashMap<String, ResourceSchema>,
    data_source_schemas: HashMap<String, DataSourceSchema>,
}

impl AdvancedProvider {
    fn new() -> Self {
        let mut resource_schemas = HashMap::new();

        // Define multiple resource types with different schemas
        resource_schemas.insert(
            "advanced_server".to_string(),
            SchemaBuilder::new()
                .attribute(
                    "name",
                    AttributeBuilder::string("name")
                        .required()
                        .description("Server name"),
                )
                .attribute(
                    "size",
                    AttributeBuilder::string("size")
                        .required()
                        .description("Server size (small, medium, large)"), // .validator(|value| {
                                                                            //     match value.as_str() {
                                                                            //         "small" | "medium" | "large" => Ok(()),
                                                                            //         _ => Err("Size must be small, medium, or large".into()),
                                                                            //     }
                                                                            // }),
                )
                .attribute(
                    "id",
                    AttributeBuilder::string("id")
                        .computed()
                        .description("Server ID"),
                )
                .attribute(
                    "status",
                    AttributeBuilder::string("status")
                        .computed()
                        .description("Server status"),
                )
                .build_resource(1),
        );

        resource_schemas.insert(
            "advanced_network".to_string(),
            SchemaBuilder::new()
                .attribute(
                    "name",
                    AttributeBuilder::string("name")
                        .required()
                        .description("Network name"),
                )
                .attribute(
                    "cidr",
                    AttributeBuilder::string("cidr")
                        .required()
                        .description("Network CIDR block"),
                )
                .attribute(
                    "id",
                    AttributeBuilder::string("id")
                        .computed()
                        .description("Network ID"),
                )
                .build_resource(1),
        );

        let mut data_source_schemas = HashMap::new();
        data_source_schemas.insert(
            "server_list".to_string(),
            SchemaBuilder::new()
                .attribute(
                    "filter",
                    AttributeBuilder::string("filter")
                        .optional()
                        .description("Filter expression"),
                )
                .attribute(
                    "servers",
                    AttributeBuilder::list(
                        "servers",
                        AttributeType::Map(Box::new(AttributeType::String)),
                    )
                    .computed()
                    .description("List of servers"),
                )
                .build_data_source(1),
        );

        Self {
            client: Arc::new(RwLock::new(None)),
            resource_schemas,
            data_source_schemas,
        }
    }
}

#[async_trait]
impl ProviderV2 for AdvancedProvider {
    async fn configure(&mut self, request: ConfigureRequest) -> ConfigureResponse {
        let mut diagnostics = Diagnostics::new();

        let endpoint = match request.config.get_string("endpoint") {
            Some(e) => e,
            None => {
                diagnostics.add_error("endpoint is required", None::<String>);
                return ConfigureResponse { diagnostics };
            }
        };

        let api_key = match request.config.get_string("api_key") {
            Some(k) => k,
            None => {
                diagnostics.add_error("api_key is required", None::<String>);
                return ConfigureResponse { diagnostics };
            }
        };

        let max_concurrent = request
            .config
            .get_number("max_concurrent_operations")
            .unwrap_or(10.0) as usize;

        let timeout_ms = request
            .config
            .get_number("operation_timeout_ms")
            .unwrap_or(5000.0) as u64;

        let config = ProviderConfig {
            api_endpoint: endpoint,
            api_key,
            max_concurrent_operations: max_concurrent,
            operation_timeout: Duration::from_millis(timeout_ms),
        };

        let client = ApiClient::new(config);
        let mut client_lock = self.client.write().await;
        *client_lock = Some(client);

        ConfigureResponse { diagnostics }
    }

    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
        match name {
            "advanced_server" => Ok(Box::new(ServerResource::new(self.client.clone()))),
            "advanced_network" => Ok(Box::new(NetworkResource::new(self.client.clone()))),
            _ => Err(format!("Unknown resource: {}", name).into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>> {
        match name {
            "server_list" => Ok(Box::new(ServerListDataSource::new(self.client.clone()))),
            _ => Err(format!("Unknown data source: {}", name).into()),
        }
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        self.resource_schemas.clone()
    }

    async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema> {
        self.data_source_schemas.clone()
    }
}

// Server resource with advanced features
struct ServerResource {
    client: Arc<RwLock<Option<ApiClient>>>,
}

impl ServerResource {
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
impl ResourceV2 for ServerResource {
    async fn schema(&self, _: SchemaRequest) -> ResourceSchemaResponse {
        ResourceSchemaResponse {
            schema: ResourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
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

        match client.create_resource("server", &name).await {
            Ok(id) => {
                let state = StateBuilder::from_config(&request.config)
                    .string("id", &id)
                    .string("status", "provisioning")
                    .build();
                CreateResponse { state, diagnostics }
            }
            Err(e) => {
                diagnostics.add_error("Failed to create server", Some(&e.to_string()));
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
                diagnostics.add_error("id is required", Some(&e.to_string()));
                return ReadResponse {
                    state: None,
                    diagnostics,
                };
            }
        };

        match client.get_resource(&id).await {
            Ok(Some(data)) => {
                let mut state = request.current_state.clone();
                if let Some(status) = data.get("status") {
                    state
                        .values
                        .insert("status".to_string(), Dynamic::String(status.clone()));
                }
                ReadResponse {
                    state: Some(state),
                    diagnostics,
                }
            }
            Ok(None) => ReadResponse {
                state: None,
                diagnostics,
            },
            Err(e) => {
                diagnostics.add_error("Failed to read server", Some(&e.to_string()));
                ReadResponse {
                    state: Some(request.current_state),
                    diagnostics,
                }
            }
        }
    }

    async fn update(&self, request: UpdateRequest) -> UpdateResponse {
        UpdateResponse {
            state: request.planned_state,
            diagnostics: Diagnostics::new(),
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
                diagnostics.add_error("id is required", Some(&e.to_string()));
                return DeleteResponse { diagnostics };
            }
        };

        if let Err(e) = client.delete_resource(&id).await {
            diagnostics.add_error("Failed to delete server", Some(&e.to_string()));
        }

        DeleteResponse { diagnostics }
    }
}

// Network resource
struct NetworkResource {
    client: Arc<RwLock<Option<ApiClient>>>,
}

impl NetworkResource {
    fn new(client: Arc<RwLock<Option<ApiClient>>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ResourceV2 for NetworkResource {
    async fn schema(&self, _: SchemaRequest) -> ResourceSchemaResponse {
        ResourceSchemaResponse {
            schema: ResourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create(&self, request: CreateRequest) -> CreateResponse {
        let mut diagnostics = Diagnostics::new();

        // Get client and use it to create the network resource
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            // Extract network name from config
            let name = request
                .config
                .get_string("name")
                .unwrap_or_else(|| format!("network-{}", uuid::Uuid::new_v4()));

            match client.create_resource("networks", &name).await {
                Ok(resource_id) => {
                    let state = StateBuilder::from_config(&request.config)
                        .string("id", &resource_id)
                        .string("name", &name)
                        .build();
                    CreateResponse { state, diagnostics }
                }
                Err(e) => {
                    diagnostics
                        .add_error(format!("Failed to create network: {}", e), None::<String>);
                    CreateResponse {
                        state: State::new(),
                        diagnostics,
                    }
                }
            }
        } else {
            diagnostics.add_error("Provider not configured".to_string(), None::<String>);
            CreateResponse {
                state: State::new(),
                diagnostics,
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> ReadResponse {
        let mut diagnostics = Diagnostics::new();

        // Get client and use it to read the network resource
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            if let Some(id) = request.current_state.get_string("id") {
                match client.get_resource(&id).await {
                    Ok(Some(resource_data)) => {
                        // Update state with fresh data from API
                        let mut state = request.current_state;
                        for (key, value) in resource_data {
                            state.values.insert(key, Dynamic::String(value));
                        }
                        ReadResponse {
                            state: Some(state),
                            diagnostics,
                        }
                    }
                    Ok(None) => {
                        // Resource no longer exists
                        ReadResponse {
                            state: None,
                            diagnostics,
                        }
                    }
                    Err(e) => {
                        diagnostics
                            .add_error(format!("Failed to read network: {}", e), None::<String>);
                        ReadResponse {
                            state: Some(request.current_state),
                            diagnostics,
                        }
                    }
                }
            } else {
                diagnostics.add_error("Network ID not found in state".to_string(), None::<String>);
                ReadResponse {
                    state: None,
                    diagnostics,
                }
            }
        } else {
            diagnostics.add_error("Provider not configured".to_string(), None::<String>);
            ReadResponse {
                state: Some(request.current_state),
                diagnostics,
            }
        }
    }

    async fn update(&self, request: UpdateRequest) -> UpdateResponse {
        let mut diagnostics = Diagnostics::new();

        // Get client and use it to update the network resource
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            if let Some(id) = request.current_state.get_string("id") {
                // In a real implementation, you would extract changes and call an update API
                // For this example, we'll simulate success and return the planned state
                println!("Updating network {} using client", id);

                // Simulate using the client for update operations
                match client.get_resource(&id).await {
                    Ok(Some(_)) => {
                        // Resource exists, proceed with update
                        UpdateResponse {
                            state: request.planned_state,
                            diagnostics,
                        }
                    }
                    Ok(None) => {
                        diagnostics.add_error(
                            "Network resource not found for update".to_string(),
                            None::<String>,
                        );
                        UpdateResponse {
                            state: request.current_state,
                            diagnostics,
                        }
                    }
                    Err(e) => {
                        diagnostics.add_error(
                            format!("Failed to verify network for update: {}", e),
                            None::<String>,
                        );
                        UpdateResponse {
                            state: request.current_state,
                            diagnostics,
                        }
                    }
                }
            } else {
                diagnostics.add_error("Network ID not found in state".to_string(), None::<String>);
                UpdateResponse {
                    state: request.current_state,
                    diagnostics,
                }
            }
        } else {
            diagnostics.add_error("Provider not configured".to_string(), None::<String>);
            UpdateResponse {
                state: request.current_state,
                diagnostics,
            }
        }
    }

    async fn delete(&self, request: DeleteRequest) -> DeleteResponse {
        let mut diagnostics = Diagnostics::new();

        // Get client and use it to delete the network resource
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            if let Some(id) = request.current_state.get_string("id") {
                match client.delete_resource(&id).await {
                    Ok(()) => {
                        // Resource successfully deleted
                        DeleteResponse { diagnostics }
                    }
                    Err(e) => {
                        diagnostics
                            .add_error(format!("Failed to delete network: {}", e), None::<String>);
                        DeleteResponse { diagnostics }
                    }
                }
            } else {
                diagnostics.add_error("Network ID not found in state".to_string(), None::<String>);
                DeleteResponse { diagnostics }
            }
        } else {
            diagnostics.add_error("Provider not configured".to_string(), None::<String>);
            DeleteResponse { diagnostics }
        }
    }
}

// Data source with filtering
struct ServerListDataSource {
    client: Arc<RwLock<Option<ApiClient>>>,
}

impl ServerListDataSource {
    fn new(client: Arc<RwLock<Option<ApiClient>>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl DataSourceV2 for ServerListDataSource {
    async fn schema(&self, _: SchemaRequest) -> DataSourceSchemaResponse {
        DataSourceSchemaResponse {
            schema: DataSourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
            diagnostics: Diagnostics::new(),
        }
    }

    async fn read(&self, request: ReadRequest) -> ReadResponse {
        let mut diagnostics = Diagnostics::new();

        // Get client and use it to list servers
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            // Extract filter from config if available
            let filter = request.current_state.get_string("filter");
            let filter_ref = filter.as_deref();

            match client.list_servers(filter_ref).await {
                Ok(server_data) => {
                    // Convert server data from API to Dynamic format
                    let mut servers = vec![];
                    for server in server_data {
                        let mut server_map = HashMap::new();
                        for (key, value) in server {
                            server_map.insert(key, Dynamic::String(value));
                        }
                        servers.push(Dynamic::Map(server_map));
                    }

                    let mut state = StateBuilder::new()
                        .list("servers", servers)
                        .string("id", "server_list");

                    // Include filter in state if it was provided
                    if let Some(f) = &filter {
                        state = state.string("filter", f);
                    }

                    ReadResponse {
                        state: Some(state.build()),
                        diagnostics,
                    }
                }
                Err(e) => {
                    diagnostics.add_error(format!("Failed to list servers: {}", e), None::<String>);
                    ReadResponse {
                        state: None,
                        diagnostics,
                    }
                }
            }
        } else {
            diagnostics.add_error("Provider not configured".to_string(), None::<String>);
            ReadResponse {
                state: None,
                diagnostics,
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Advanced Provider V2 Example\n");

    // Create and configure provider
    let mut provider = AdvancedProvider::new();

    let mut config = Config::new();
    config.values.insert(
        "endpoint".to_string(),
        Dynamic::String("https://api.example.com".to_string()),
    );
    config.values.insert(
        "api_key".to_string(),
        Dynamic::String("secret-key".to_string()),
    );
    config.values.insert(
        "max_concurrent_operations".to_string(),
        Dynamic::Number(5.0),
    );
    config
        .values
        .insert("operation_timeout_ms".to_string(), Dynamic::Number(2000.0));

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

    println!("Provider configured successfully!");

    // Create multiple server resources concurrently
    println!("\nCreating servers concurrently with rate limiting...");
    let server_resource = Arc::new(provider.create_resource("advanced_server").await.unwrap());

    let mut server_handles = vec![];
    for i in 0..10 {
        let resource = server_resource.clone();
        let handle = tokio::spawn(async move {
            let start = tokio::time::Instant::now();

            let mut config = Config::new();
            config
                .values
                .insert("name".to_string(), Dynamic::String(format!("server-{}", i)));
            config
                .values
                .insert("size".to_string(), Dynamic::String("medium".to_string()));

            let response = resource
                .create(CreateRequest {
                    context: tfplug::context::Context::new(),
                    config,
                    planned_state: State::new(),
                })
                .await;

            let elapsed = start.elapsed();

            if response.diagnostics.errors.is_empty() {
                println!(
                    "  Server {} created in {:?}: ID = {:?}",
                    i,
                    elapsed,
                    response.state.get_string("id")
                );
                Some(response.state)
            } else {
                eprintln!(
                    "  Failed to create server {}: {:?}",
                    i, response.diagnostics.errors
                );
                None
            }
        });
        server_handles.push(handle);
    }

    let server_states: Vec<State> = futures::future::join_all(server_handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .filter_map(|s| s)
        .collect();

    println!(
        "\nCreated {} servers (rate limited to 5 concurrent)",
        server_states.len()
    );

    // Test data source
    println!("\nTesting data source with filtering...");
    let data_source = provider.create_data_source("server_list").await.unwrap();

    let mut ds_config = Config::new();
    ds_config
        .values
        .insert("filter".to_string(), Dynamic::String("1".to_string()));

    let ds_response = data_source
        .read(ReadRequest {
            context: tfplug::context::Context::new(),
            current_state: State::new(),
        })
        .await;

    if let Some(state) = ds_response.state {
        if let Some(Dynamic::List(servers)) = state.values.get("servers") {
            println!("Found {} servers matching filter '1':", servers.len());
            for server in servers {
                if let Dynamic::Map(s) = server {
                    println!("  - {:?}", s.get("name"));
                }
            }
        }
    }
}
