//! Comprehensive test suite for async provider traits

#![allow(clippy::disallowed_methods)] // Allow unwrap() in tests for clarity

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;

use tfplug::provider::{DataSourceSchema, ResourceSchema};
use tfplug::request::{
    ConfigureRequest, ConfigureResponse, CreateRequest, CreateResponse, DataSourceSchemaResponse,
    DeleteRequest, DeleteResponse, ReadRequest, ReadResponse, ResourceSchemaResponse,
    SchemaRequest, UpdateRequest, UpdateResponse,
};
use tfplug::types::{Config, Diagnostics, Dynamic, State};
use tfplug::Result;
use tfplug::{DataSourceV2, ProviderV2, ResourceV2};

// Track concurrent operations
#[derive(Default)]
struct OperationStats {
    concurrent_creates: AtomicUsize,
    max_concurrent: AtomicUsize,
    total_operations: AtomicUsize,
}

impl OperationStats {
    fn start_operation(&self) -> usize {
        let current = self.concurrent_creates.fetch_add(1, Ordering::SeqCst) + 1;
        let mut max = self.max_concurrent.load(Ordering::SeqCst);
        while current > max {
            match self.max_concurrent.compare_exchange(
                max,
                current,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }
        self.total_operations.fetch_add(1, Ordering::SeqCst);
        current
    }

    fn end_operation(&self) {
        self.concurrent_creates.fetch_sub(1, Ordering::SeqCst);
    }
}

// Advanced provider with state tracking
struct AdvancedProvider {
    config: RwLock<Option<Config>>,
    stats: Arc<OperationStats>,
}

impl AdvancedProvider {
    fn new() -> Self {
        Self {
            config: RwLock::new(None),
            stats: Arc::new(OperationStats::default()),
        }
    }
}

#[async_trait]
impl ProviderV2 for AdvancedProvider {
    async fn configure(&mut self, request: ConfigureRequest) -> ConfigureResponse {
        // Simulate async configuration work
        sleep(Duration::from_millis(10)).await;

        let mut config = self.config.write().await;
        *config = Some(request.config);

        ConfigureResponse {
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
        match name {
            "tracked" => Ok(Box::new(TrackedResource {
                stats: self.stats.clone(),
            })),
            "stateful" => Ok(Box::new(StatefulResource::new())),
            _ => Err(format!("Unknown resource: {}", name).into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>> {
        match name {
            "config_reader" => {
                let config = self.config.read().await;
                Ok(Box::new(ConfigReaderDataSource {
                    provider_config: config.clone(),
                }))
            }
            _ => Err(format!("Unknown data source: {}", name).into()),
        }
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        // Simulate async schema generation
        sleep(Duration::from_millis(1)).await;

        let mut schemas = HashMap::new();
        schemas.insert(
            "tracked".to_string(),
            ResourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
        );
        schemas.insert(
            "stateful".to_string(),
            ResourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
        );
        schemas
    }

    async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema> {
        let mut schemas = HashMap::new();
        schemas.insert(
            "config_reader".to_string(),
            DataSourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
        );
        schemas
    }
}

// Resource that tracks concurrent operations
struct TrackedResource {
    stats: Arc<OperationStats>,
}

#[async_trait]
impl ResourceV2 for TrackedResource {
    async fn schema(&self, _request: SchemaRequest) -> ResourceSchemaResponse {
        ResourceSchemaResponse {
            schema: ResourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create(&self, _request: CreateRequest) -> CreateResponse {
        let current = self.stats.start_operation();

        // Simulate work
        sleep(Duration::from_millis(50)).await;

        let mut state = State::new();
        state.values.insert(
            "concurrent_operations".to_string(),
            Dynamic::Number(current as f64),
        );
        state.values.insert(
            "max_concurrent".to_string(),
            Dynamic::Number(self.stats.max_concurrent.load(Ordering::SeqCst) as f64),
        );

        self.stats.end_operation();

        CreateResponse {
            state,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn read(&self, request: ReadRequest) -> ReadResponse {
        ReadResponse {
            state: Some(request.current_state),
            diagnostics: Diagnostics::new(),
        }
    }

    async fn update(&self, _request: UpdateRequest) -> UpdateResponse {
        UpdateResponse {
            state: State::new(),
            diagnostics: Diagnostics::new(),
        }
    }

    async fn delete(&self, _request: DeleteRequest) -> DeleteResponse {
        DeleteResponse {
            diagnostics: Diagnostics::new(),
        }
    }
}

// Resource with internal state
struct StatefulResource {
    state: RwLock<HashMap<String, String>>,
}

impl StatefulResource {
    fn new() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ResourceV2 for StatefulResource {
    async fn schema(&self, _request: SchemaRequest) -> ResourceSchemaResponse {
        ResourceSchemaResponse {
            schema: ResourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create(&self, request: CreateRequest) -> CreateResponse {
        let mut internal_state = self.state.write().await;

        // Store config values in internal state
        for (key, value) in &request.config.values {
            if let Some(str_val) = value.as_string() {
                internal_state.insert(key.clone(), str_val.to_string());
            }
        }

        let mut state = State::new();
        state.values.insert(
            "id".to_string(),
            Dynamic::String("stateful-resource".to_string()),
        );
        state.values.insert(
            "item_count".to_string(),
            Dynamic::Number(internal_state.len() as f64),
        );

        CreateResponse {
            state,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn read(&self, _request: ReadRequest) -> ReadResponse {
        let internal_state = self.state.read().await;

        let mut state = State::new();
        state.values.insert(
            "id".to_string(),
            Dynamic::String("stateful-resource".to_string()),
        );
        state.values.insert(
            "item_count".to_string(),
            Dynamic::Number(internal_state.len() as f64),
        );

        ReadResponse {
            state: Some(state),
            diagnostics: Diagnostics::new(),
        }
    }

    async fn update(&self, request: UpdateRequest) -> UpdateResponse {
        let mut internal_state = self.state.write().await;
        internal_state.clear();

        // Update with new config values
        for (key, value) in &request.config.values {
            if let Some(str_val) = value.as_string() {
                internal_state.insert(key.clone(), str_val.to_string());
            }
        }

        let mut state = State::new();
        state.values.insert(
            "id".to_string(),
            Dynamic::String("stateful-resource".to_string()),
        );
        state.values.insert(
            "item_count".to_string(),
            Dynamic::Number(internal_state.len() as f64),
        );

        UpdateResponse {
            state,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn delete(&self, _request: DeleteRequest) -> DeleteResponse {
        let mut internal_state = self.state.write().await;
        internal_state.clear();

        DeleteResponse {
            diagnostics: Diagnostics::new(),
        }
    }
}

// Data source that reads provider configuration
struct ConfigReaderDataSource {
    provider_config: Option<Config>,
}

#[async_trait]
impl DataSourceV2 for ConfigReaderDataSource {
    async fn schema(&self, _request: SchemaRequest) -> DataSourceSchemaResponse {
        DataSourceSchemaResponse {
            schema: DataSourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
            diagnostics: Diagnostics::new(),
        }
    }

    async fn read(&self, _request: ReadRequest) -> ReadResponse {
        let mut state = State::new();

        if let Some(config) = &self.provider_config {
            state
                .values
                .insert("has_config".to_string(), Dynamic::Bool(true));
            state.values.insert(
                "config_keys".to_string(),
                Dynamic::Number(config.values.len() as f64),
            );
        } else {
            state
                .values
                .insert("has_config".to_string(), Dynamic::Bool(false));
        }

        ReadResponse {
            state: Some(state),
            diagnostics: Diagnostics::new(),
        }
    }
}

#[tokio::test]
async fn test_provider_configuration_is_async() {
    let mut provider = AdvancedProvider::new();

    let mut config = Config::new();
    config
        .values
        .insert("key".to_string(), Dynamic::String("value".to_string()));

    let start = Instant::now();
    let resp = provider
        .configure(ConfigureRequest {
            context: tfplug::context::Context::new(),
            config,
        })
        .await;
    let elapsed = start.elapsed();

    assert_eq!(resp.diagnostics.errors.len(), 0);
    // Should take at least 10ms due to sleep
    assert!(elapsed.as_millis() >= 10);
}

#[tokio::test]
async fn test_concurrent_resource_operations_are_tracked() {
    let provider = Arc::new(AdvancedProvider::new());
    let mut handles = vec![];

    // Spawn 10 concurrent operations
    for _ in 0..10 {
        let provider_clone = provider.clone();
        let handle = task::spawn(async move {
            let resource = provider_clone.create_resource("tracked").await.unwrap();
            let resp = resource
                .create(CreateRequest {
                    context: tfplug::context::Context::new(),
                    config: Config::new(),
                    planned_state: State::new(),
                })
                .await;

            let concurrent = resp.state.get_number("concurrent_operations").unwrap();
            let max = resp.state.get_number("max_concurrent").unwrap();
            (concurrent, max)
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // Check that we saw concurrency
    let max_concurrent = results.iter().map(|(_, max)| *max).fold(0.0, f64::max);
    assert!(
        max_concurrent > 1.0,
        "Expected concurrent operations, but max was {}",
        max_concurrent
    );

    // Verify all operations completed
    assert_eq!(provider.stats.total_operations.load(Ordering::SeqCst), 10);
}

#[tokio::test]
async fn test_stateful_resource_maintains_state() {
    let provider = AdvancedProvider::new();
    let resource = provider.create_resource("stateful").await.unwrap();

    // Create with initial config
    let mut config = Config::new();
    config
        .values
        .insert("key1".to_string(), Dynamic::String("value1".to_string()));
    config
        .values
        .insert("key2".to_string(), Dynamic::String("value2".to_string()));

    let create_resp = resource
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config,
            planned_state: State::new(),
        })
        .await;

    assert_eq!(create_resp.state.get_number("item_count").unwrap(), 2.0);

    // Read should return same count
    let read_resp = resource
        .read(ReadRequest {
            context: tfplug::context::Context::new(),
            current_state: State::new(),
        })
        .await;

    assert_eq!(
        read_resp.state.unwrap().get_number("item_count").unwrap(),
        2.0
    );

    // Update with new config
    let mut new_config = Config::new();
    new_config
        .values
        .insert("key3".to_string(), Dynamic::String("value3".to_string()));

    let update_resp = resource
        .update(UpdateRequest {
            context: tfplug::context::Context::new(),
            config: new_config,
            current_state: State::new(),
            planned_state: State::new(),
        })
        .await;

    assert_eq!(update_resp.state.get_number("item_count").unwrap(), 1.0);
}

#[tokio::test]
async fn test_data_source_can_access_provider_config() {
    let mut provider = AdvancedProvider::new();

    // Configure provider
    let mut config = Config::new();
    config
        .values
        .insert("api_key".to_string(), Dynamic::String("secret".to_string()));

    provider
        .configure(ConfigureRequest {
            context: tfplug::context::Context::new(),
            config,
        })
        .await;

    // Create data source after configuration
    let data_source = provider.create_data_source("config_reader").await.unwrap();

    let resp = data_source
        .read(ReadRequest {
            context: tfplug::context::Context::new(),
            current_state: State::new(),
        })
        .await;

    let state = resp.state.unwrap();
    assert!(state.get_bool("has_config").unwrap());
    assert_eq!(state.get_number("config_keys").unwrap(), 1.0);
}

#[tokio::test]
async fn test_factory_methods_are_async() {
    let provider = AdvancedProvider::new();

    // Test that factory methods can do async work
    let start = Instant::now();
    let _schemas = provider.resource_schemas().await;
    let elapsed = start.elapsed();

    // Should take at least 1ms due to sleep
    assert!(elapsed.as_millis() >= 1);
}
