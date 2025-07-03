//! Simple test to verify async traits work correctly in provider

#![allow(clippy::disallowed_methods)] // Allow unwrap() in tests for clarity

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;
use tokio::time::sleep;

// Import the async traits from provider
use tfplug::provider::{DataSourceSchema, ResourceSchema};
use tfplug::request::{
    ConfigureRequest, ConfigureResponse, CreateRequest, CreateResponse, DataSourceSchemaResponse,
    DeleteRequest, DeleteResponse, ReadRequest, ReadResponse, ResourceSchemaResponse,
    SchemaRequest, UpdateRequest, UpdateResponse,
};
use tfplug::types::{Config, Diagnostics, State};
use tfplug::Result;
use tfplug::{DataSourceV2, ProviderV2, ResourceV2};

// Simple provider implementation
struct SimpleProvider;

#[async_trait]
impl ProviderV2 for SimpleProvider {
    async fn configure(&mut self, _request: ConfigureRequest) -> ConfigureResponse {
        // Simulate async work
        sleep(Duration::from_millis(1)).await;
        ConfigureResponse {
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
        match name {
            "simple" => Ok(Box::new(SimpleResource)),
            "slow" => Ok(Box::new(SlowResource)),
            _ => Err("Unknown resource".into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>> {
        match name {
            "simple" => Ok(Box::new(SimpleDataSource)),
            _ => Err("Unknown data source".into()),
        }
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        let mut schemas = HashMap::new();
        schemas.insert(
            "simple".to_string(),
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
            "simple".to_string(),
            DataSourceSchema {
                version: 1,
                attributes: HashMap::new(),
            },
        );
        schemas
    }
}

struct SimpleResource;

#[async_trait]
impl ResourceV2 for SimpleResource {
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
        // Simulate async work
        sleep(Duration::from_millis(1)).await;
        CreateResponse {
            state: State::new(),
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

struct SlowResource;

#[async_trait]
impl ResourceV2 for SlowResource {
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
        // Simulate slow async operation
        sleep(Duration::from_millis(100)).await;
        CreateResponse {
            state: State::new(),
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

struct SimpleDataSource;

#[async_trait]
impl DataSourceV2 for SimpleDataSource {
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
        ReadResponse {
            state: Some(State::new()),
            diagnostics: Diagnostics::new(),
        }
    }
}

#[tokio::test]
async fn test_async_trait_methods_work() {
    let mut provider = SimpleProvider;

    // Test configure
    let config_resp = provider
        .configure(ConfigureRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
        })
        .await;
    assert_eq!(config_resp.diagnostics.errors.len(), 0);

    // Test factory methods
    let resource = provider.create_resource("simple").await.unwrap();
    let data_source = provider.create_data_source("simple").await.unwrap();

    // Test resource methods
    let create_resp = resource
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
            planned_state: State::new(),
        })
        .await;
    assert_eq!(create_resp.diagnostics.errors.len(), 0);

    // Test data source methods
    let read_resp = data_source
        .read(ReadRequest {
            context: tfplug::context::Context::new(),
            current_state: State::new(),
        })
        .await;
    assert!(read_resp.state.is_some());
}

#[tokio::test]
async fn test_concurrent_async_operations() {
    let provider = Arc::new(SimpleProvider);
    let start = Instant::now();

    let mut handles = vec![];

    // Spawn 5 concurrent tasks creating slow resources
    for _ in 0..5 {
        let provider_clone = provider.clone();
        let handle = task::spawn(async move {
            let resource = provider_clone.create_resource("slow").await.unwrap();
            let _resp = resource
                .create(CreateRequest {
                    context: tfplug::context::Context::new(),
                    config: Config::new(),
                    planned_state: State::new(),
                })
                .await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();

    // If operations were serialized, this would take 5 * 100ms = 500ms
    // With concurrent execution, should be close to 100ms
    assert!(
        elapsed.as_millis() < 200,
        "Concurrent operations took too long: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_async_error_handling() {
    let provider = SimpleProvider;

    // Test unknown resource error
    let result = provider.create_resource("unknown").await;
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("Unknown resource"));

    // Test unknown data source error
    let result = provider.create_data_source("unknown").await;
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("Unknown data source"));
}

#[tokio::test]
async fn test_factory_creates_new_instances() {
    let provider = SimpleProvider;

    // Create multiple instances
    let resource1 = provider.create_resource("simple").await.unwrap();
    let resource2 = provider.create_resource("simple").await.unwrap();

    // Both should succeed
    let resp1 = resource1
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
            planned_state: State::new(),
        })
        .await;

    let resp2 = resource2
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
            planned_state: State::new(),
        })
        .await;

    assert_eq!(resp1.diagnostics.errors.len(), 0);
    assert_eq!(resp2.diagnostics.errors.len(), 0);
}
