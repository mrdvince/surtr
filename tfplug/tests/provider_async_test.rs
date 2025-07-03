#![allow(clippy::disallowed_methods)] // Allow unwrap() in tests for clarity

use async_trait::async_trait;
use std::collections::HashMap;
use tfplug::provider::{DataSourceSchema, ResourceSchema};
use tfplug::request::{
    ConfigureRequest, ConfigureResponse, CreateRequest, CreateResponse, DataSourceSchemaResponse,
    DeleteRequest, DeleteResponse, ReadRequest, ReadResponse, ResourceSchemaResponse,
    SchemaRequest, UpdateRequest, UpdateResponse,
};
use tfplug::types::{Config, Diagnostics, State};
use tfplug::Result;
use tfplug::{DataSourceV2, ProviderV2, ResourceV2};

struct TestProvider;

#[async_trait]
impl ProviderV2 for TestProvider {
    async fn configure(&mut self, _request: ConfigureRequest) -> ConfigureResponse {
        ConfigureResponse {
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
        match name {
            "test" => Ok(Box::new(TestResource)),
            _ => Err("Unknown resource".into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>> {
        match name {
            "test" => Ok(Box::new(TestDataSource)),
            _ => Err("Unknown data source".into()),
        }
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        HashMap::new()
    }

    async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema> {
        HashMap::new()
    }
}

struct TestResource;

#[async_trait]
impl ResourceV2 for TestResource {
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

struct TestDataSource;

#[async_trait]
impl DataSourceV2 for TestDataSource {
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
async fn test_async_provider_works() {
    let mut provider = TestProvider;
    let _config_resp = provider
        .configure(ConfigureRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
        })
        .await;

    let resource = provider.create_resource("test").await.unwrap();
    let create_resp = resource
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
            planned_state: State::new(),
        })
        .await;

    assert_eq!(create_resp.diagnostics.errors.len(), 0);
}

#[tokio::test]
async fn test_concurrent_resource_creation() {
    use std::sync::Arc;
    use tokio::task;

    let provider = Arc::new(TestProvider);
    let mut handles = vec![];

    for _ in 0..5 {
        let provider_clone = provider.clone();
        let handle = task::spawn(async move {
            let resource = provider_clone.create_resource("test").await.unwrap();
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

    for handle in handles {
        handle.await.unwrap();
    }
}
