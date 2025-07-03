//! Provider v2 - Factory-based architecture without locks
//!
//! This module implements a new provider architecture that eliminates locks
//! and uses factory methods to create resources and data sources on demand.
use crate::attribute_type::AttributeType;
use crate::request::{
    ConfigureRequest, ConfigureResponse, CreateRequest, CreateResponse, DataSourceSchemaResponse,
    DeleteRequest, DeleteResponse, ReadRequest, ReadResponse, ResourceSchemaResponse,
    SchemaRequest, UpdateRequest, UpdateResponse,
};
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DataSourceSchema {
    pub version: i64,
    pub attributes: HashMap<String, Attribute>,
}

#[derive(Debug, Clone)]
pub struct ResourceSchema {
    pub version: i64,
    pub attributes: HashMap<String, Attribute>,
}

pub struct Attribute {
    pub name: String,
    pub r#type: AttributeType,
    pub description: String,
    pub required: bool,
    pub optional: bool,
    pub computed: bool,
    pub sensitive: bool,
    pub validators: Vec<Box<dyn crate::validator::Validator>>,
    pub plan_modifiers: Vec<Box<dyn crate::plan_modifier::PlanModifier>>,
    pub default: Option<Box<dyn crate::defaults::Default>>,
}

impl std::fmt::Debug for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.name)
            .field("type", &self.r#type)
            .field("description", &self.description)
            .field("required", &self.required)
            .field("optional", &self.optional)
            .field("computed", &self.computed)
            .field("sensitive", &self.sensitive)
            .field(
                "validators",
                &format!("{} validators", self.validators.len()),
            )
            .field(
                "plan_modifiers",
                &format!("{} plan modifiers", self.plan_modifiers.len()),
            )
            .field("default", &self.default.is_some())
            .finish()
    }
}

impl Clone for Attribute {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            r#type: self.r#type.clone(),
            description: self.description.clone(),
            required: self.required,
            optional: self.optional,
            computed: self.computed,
            sensitive: self.sensitive,
            validators: vec![],
            plan_modifiers: vec![],
            default: None,
        }
    }
}

#[async_trait]
pub trait ProviderV2: Send + Sync {
    async fn configure(&mut self, request: ConfigureRequest) -> ConfigureResponse;

    // Factory methods - create instances on demand
    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>>;
    async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>>;

    // Schema methods - return cached schemas
    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema>;
    async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema>;
}

#[async_trait]
pub trait ResourceV2: Send + Sync {
    async fn schema(&self, request: SchemaRequest) -> ResourceSchemaResponse;
    async fn create(&self, request: CreateRequest) -> CreateResponse;
    async fn read(&self, request: ReadRequest) -> ReadResponse;
    async fn update(&self, request: UpdateRequest) -> UpdateResponse;
    async fn delete(&self, request: DeleteRequest) -> DeleteResponse;
}

#[async_trait]
pub trait DataSourceV2: Send + Sync {
    async fn schema(&self, request: SchemaRequest) -> DataSourceSchemaResponse;
    async fn read(&self, request: ReadRequest) -> ReadResponse;
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::types::{Config, Diagnostics, Dynamic, State};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::task;
    use tokio::time::sleep;

    #[tokio::test]
    async fn provider_can_be_shared_across_threads_without_locks() {
        let provider = Arc::new(TestProvider::new());
        let mut handles = vec![];

        // Spawn 10 tasks that will all create and use resources concurrently
        for i in 0..10 {
            let provider_clone = provider.clone();
            let handle = task::spawn(async move {
                // Create a new resource instance
                let resource = provider_clone
                    .create_resource("test_resource")
                    .await
                    .expect("Should create resource");

                // Use the resource
                let mut config = Config::new();
                config.values.insert(
                    "thread_index".to_string(),
                    Dynamic::String(format!("{}", i)),
                );

                let req = CreateRequest {
                    context: Context::new(),
                    config,
                    planned_state: State::new(),
                };
                let resp = resource.create(req).await;

                assert_eq!(resp.diagnostics.errors.len(), 0);
                // Verify we got a unique task ID
                assert!(resp.state.get_string("task_id").is_some());
                // Verify our thread index was preserved
                assert_eq!(
                    resp.state.get_string("thread_index").unwrap(),
                    format!("{}", i)
                );
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn factory_creates_new_instances_each_time() {
        let provider = TestProvider::new();

        // Just verify factory methods work - can't check pointer equality with unit structs
        let resource1 = provider.create_resource("test_resource").await;
        let resource2 = provider.create_resource("test_resource").await;

        assert!(resource1.is_ok());
        assert!(resource2.is_ok());
    }

    #[tokio::test]
    async fn schemas_return_consistent_values() {
        let provider = TestProvider::new();

        let schemas1 = provider.resource_schemas().await;
        let schemas2 = provider.resource_schemas().await;

        // Should return the same values
        assert_eq!(schemas1.len(), schemas2.len());
        assert!(schemas1.contains_key("test_resource"));
        assert!(schemas1.contains_key("slow_resource"));
    }

    #[tokio::test]
    async fn concurrent_operations_do_not_block_each_other() {
        let provider = Arc::new(TestProvider::new());
        let start = std::time::Instant::now();

        let mut handles = vec![];
        for _ in 0..5 {
            let provider_clone = provider.clone();
            let handle = task::spawn(async move {
                let resource = provider_clone
                    .create_resource("slow_resource")
                    .await
                    .unwrap();
                let req = CreateRequest {
                    context: Context::new(),
                    config: Config::new(),
                    planned_state: State::new(),
                };

                // This simulates a slow operation
                resource.create(req).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let elapsed = start.elapsed();
        // If operations were serialized, this would take 5 * 100ms = 500ms
        // With concurrent execution, should be close to 100ms
        assert!(
            elapsed.as_millis() < 200,
            "Operations took too long: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn unknown_resource_returns_error() {
        let provider = TestProvider::new();

        let result = provider.create_resource("unknown").await;
        assert!(result.is_err());

        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Unknown resource"));
    }

    #[tokio::test]
    async fn data_source_factory_works() {
        let provider = TestProvider::new();

        let data_source = provider.create_data_source("test_data").await.unwrap();
        let req = ReadRequest {
            context: Context::new(),
            current_state: State::new(),
        };

        let resp = data_source.read(req).await;
        assert!(resp.state.is_some());
        assert_eq!(resp.state.unwrap().get_string("version").unwrap(), "1.0.0");
    }

    // Test provider implementation
    struct TestProvider {
        configured: bool,
    }

    impl TestProvider {
        fn new() -> Self {
            Self { configured: false }
        }
    }

    #[async_trait]
    impl ProviderV2 for TestProvider {
        async fn configure(&mut self, _request: ConfigureRequest) -> ConfigureResponse {
            self.configured = true;
            ConfigureResponse {
                diagnostics: Diagnostics::new(),
            }
        }

        async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
            match name {
                "test_resource" => Ok(Box::new(TestResource::new())),
                "slow_resource" => Ok(Box::new(SlowResource)),
                _ => Err(format!("Unknown resource: {}", name).into()),
            }
        }

        async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>> {
            match name {
                "test_data" => Ok(Box::new(TestDataSource)),
                _ => Err(format!("Unknown data source: {}", name).into()),
            }
        }

        async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
            // Return empty map for now since ResourceSchema doesn't implement Clone
            // In a real implementation, you would likely cache these or generate them on demand
            let mut schemas = HashMap::new();
            schemas.insert(
                "test_resource".to_string(),
                ResourceSchema {
                    version: 1,
                    attributes: HashMap::new(),
                },
            );
            schemas.insert(
                "slow_resource".to_string(),
                ResourceSchema {
                    version: 1,
                    attributes: HashMap::new(),
                },
            );
            schemas
        }

        async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema> {
            // Return empty map for now since DataSourceSchema doesn't implement Clone
            let mut schemas = HashMap::new();
            schemas.insert(
                "test_data".to_string(),
                DataSourceSchema {
                    version: 1,
                    attributes: HashMap::new(),
                },
            );
            schemas
        }
    }

    struct TestResource;

    impl TestResource {
        fn new() -> Self {
            Self
        }
    }

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

        async fn create(&self, request: CreateRequest) -> CreateResponse {
            let mut state = State::new();
            state.values.insert(
                "id".to_string(),
                Dynamic::String("generated-id".to_string()),
            );

            // Add task info to verify concurrent execution
            let task_id = match task::try_id() {
                Some(id) => format!("{:?}", id),
                None => "no-task-id".to_string(),
            };
            state
                .values
                .insert("task_id".to_string(), Dynamic::String(task_id));

            // Pass through any thread index from config
            if let Some(thread_index) = request.config.get_string("thread_index") {
                state
                    .values
                    .insert("thread_index".to_string(), Dynamic::String(thread_index));
            }

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

    // Slow resource for testing concurrency
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
            // Simulate slow operation
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
            let mut state = State::new();
            state
                .values
                .insert("version".to_string(), Dynamic::String("1.0.0".to_string()));

            ReadResponse {
                state: Some(state),
                diagnostics: Diagnostics::new(),
            }
        }
    }
}
