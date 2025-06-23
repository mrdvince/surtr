//! Tests for error handling and edge cases in ProviderV2 async architecture

#![allow(clippy::disallowed_methods)] // Allow unwrap() in tests for clarity

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tfplug::provider::ResourceSchema;
use tfplug::request::{ResourceSchemaResponse, *};
use tfplug::types::{Config, Diagnostics, Dynamic, State};
use tfplug::{AttributeBuilder, Result, SchemaBuilder, StateBuilder};
use tfplug::{ProviderV2, ResourceV2};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;

struct TimeoutProvider {
    operation_timeout: Duration,
    resource_schemas: HashMap<String, ResourceSchema>,
}

impl TimeoutProvider {
    fn new(timeout_ms: u64) -> Self {
        let mut resource_schemas = HashMap::new();
        resource_schemas.insert(
            "timeout_resource".to_string(),
            SchemaBuilder::new()
                .attribute(
                    "delay_ms",
                    AttributeBuilder::number("delay_ms")
                        .required()
                        .description("Operation delay in milliseconds"),
                )
                .build_resource(1),
        );

        Self {
            operation_timeout: Duration::from_millis(timeout_ms),
            resource_schemas,
        }
    }
}

#[async_trait]
impl ProviderV2 for TimeoutProvider {
    async fn configure(&mut self, _: ConfigureRequest) -> ConfigureResponse {
        ConfigureResponse {
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
        match name {
            "timeout_resource" => Ok(Box::new(TimeoutResource::new(self.operation_timeout))),
            _ => Err(format!("Unknown resource: {}", name).into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> Result<Box<dyn tfplug::DataSourceV2>> {
        Err(format!("Unknown data source: {}", name).into())
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        self.resource_schemas.clone()
    }

    async fn data_source_schemas(&self) -> HashMap<String, tfplug::provider::DataSourceSchema> {
        HashMap::new()
    }
}

struct TimeoutResource {
    operation_timeout: Duration,
}

impl TimeoutResource {
    fn new(timeout: Duration) -> Self {
        Self {
            operation_timeout: timeout,
        }
    }
}

#[async_trait]
impl ResourceV2 for TimeoutResource {
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

        let delay_ms = match request.config.get_number("delay_ms") {
            Some(d) => d as u64,
            None => {
                diagnostics.add_error("delay_ms is required", None::<String>);
                return CreateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }
        };

        let operation = async {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            StateBuilder::from_config(&request.config)
                .string("id", "test-id")
                .build()
        };

        match timeout(self.operation_timeout, operation).await {
            Ok(state) => CreateResponse { state, diagnostics },
            Err(_) => {
                diagnostics.add_error(
                    "Operation timed out",
                    Some(&format!(
                        "Operation took longer than {} ms",
                        self.operation_timeout.as_millis()
                    )),
                );
                CreateResponse {
                    state: request.planned_state,
                    diagnostics,
                }
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> ReadResponse {
        ReadResponse {
            state: Some(request.current_state),
            diagnostics: Diagnostics::new(),
        }
    }

    async fn update(&self, request: UpdateRequest) -> UpdateResponse {
        UpdateResponse {
            state: request.planned_state,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn delete(&self, _: DeleteRequest) -> DeleteResponse {
        DeleteResponse {
            diagnostics: Diagnostics::new(),
        }
    }
}

struct RateLimitedProvider {
    semaphore: Arc<Semaphore>,
    resource_schemas: HashMap<String, ResourceSchema>,
}

impl RateLimitedProvider {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            resource_schemas: HashMap::new(),
        }
    }
}

#[async_trait]
impl ProviderV2 for RateLimitedProvider {
    async fn configure(&mut self, _: ConfigureRequest) -> ConfigureResponse {
        ConfigureResponse {
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
        match name {
            "rate_limited" => Ok(Box::new(RateLimitedResource::new(self.semaphore.clone()))),
            _ => Err(format!("Unknown resource: {}", name).into()),
        }
    }

    async fn create_data_source(&self, name: &str) -> Result<Box<dyn tfplug::DataSourceV2>> {
        Err(format!("Unknown data source: {}", name).into())
    }

    async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
        self.resource_schemas.clone()
    }

    async fn data_source_schemas(&self) -> HashMap<String, tfplug::provider::DataSourceSchema> {
        HashMap::new()
    }
}

struct RateLimitedResource {
    semaphore: Arc<Semaphore>,
}

impl RateLimitedResource {
    fn new(semaphore: Arc<Semaphore>) -> Self {
        Self { semaphore }
    }
}

#[async_trait]
impl ResourceV2 for RateLimitedResource {
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

        // Acquire semaphore permit
        let _permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => {
                diagnostics.add_error("Failed to acquire rate limit permit", None::<String>);
                return CreateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }
        };

        // Simulate work
        tokio::time::sleep(Duration::from_millis(50)).await;

        let state = StateBuilder::from_config(&request.config)
            .string("id", format!("rate-limited-{}", uuid::Uuid::new_v4()))
            .build();

        CreateResponse { state, diagnostics }
    }

    async fn read(&self, request: ReadRequest) -> ReadResponse {
        ReadResponse {
            state: Some(request.current_state),
            diagnostics: Diagnostics::new(),
        }
    }

    async fn update(&self, request: UpdateRequest) -> UpdateResponse {
        UpdateResponse {
            state: request.planned_state,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn delete(&self, _: DeleteRequest) -> DeleteResponse {
        DeleteResponse {
            diagnostics: Diagnostics::new(),
        }
    }
}

#[tokio::test]
async fn test_operation_timeout() {
    let mut provider = TimeoutProvider::new(100); // 100ms timeout
    provider
        .configure(ConfigureRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
        })
        .await;

    let resource = provider.create_resource("timeout_resource").await.unwrap();

    // Test operation that completes within timeout
    let mut config = Config::new();
    config
        .values
        .insert("delay_ms".to_string(), Dynamic::Number(50.0)); // 50ms delay

    let response = resource
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config: config.clone(),
            planned_state: State::new(),
        })
        .await;

    assert!(response.diagnostics.errors.is_empty());
    assert!(response.state.get_string("id").is_some());

    // Test operation that exceeds timeout
    config
        .values
        .insert("delay_ms".to_string(), Dynamic::Number(200.0)); // 200ms delay

    let response = resource
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config,
            planned_state: State::new(),
        })
        .await;

    assert!(!response.diagnostics.errors.is_empty());
    assert!(response.diagnostics.errors[0].summary.contains("timed out"));
}

#[tokio::test]
async fn test_rate_limiting() {
    let mut provider = RateLimitedProvider::new(2); // Allow 2 concurrent operations
    provider
        .configure(ConfigureRequest {
            context: tfplug::context::Context::new(),
            config: Config::new(),
        })
        .await;

    let resource = Arc::new(provider.create_resource("rate_limited").await.unwrap());

    // Start 4 concurrent operations (only 2 should run at a time)
    let start = tokio::time::Instant::now();
    let mut handles = vec![];

    for i in 0..4 {
        let resource_clone = resource.clone();
        let handle = tokio::spawn(async move {
            let config = Config::new();
            let response = resource_clone
                .create(CreateRequest {
                    context: tfplug::context::Context::new(),
                    config,
                    planned_state: State::new(),
                })
                .await;

            assert!(response.diagnostics.errors.is_empty());
            (i, tokio::time::Instant::now())
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    let end_times: Vec<_> = results
        .into_iter()
        .filter_map(|r| r.ok())
        .map(|(_, time)| time)
        .collect();

    // With 2 concurrent operations and 50ms each, 4 operations should take ~100ms
    let total_duration = end_times.iter().max().unwrap().duration_since(start);
    assert!(total_duration.as_millis() >= 100);
    assert!(total_duration.as_millis() < 150); // Some buffer for test execution
}

#[tokio::test]
async fn test_panic_recovery() {
    struct PanicResource;

    #[async_trait]
    impl ResourceV2 for PanicResource {
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
            if request.config.get_bool("should_panic").unwrap_or(false) {
                panic!("Intentional panic for testing");
            }

            CreateResponse {
                state: State::new(),
                diagnostics: Diagnostics::new(),
            }
        }

        async fn read(&self, _: ReadRequest) -> ReadResponse {
            ReadResponse {
                state: Some(State::new()),
                diagnostics: Diagnostics::new(),
            }
        }

        async fn update(&self, request: UpdateRequest) -> UpdateResponse {
            UpdateResponse {
                state: request.planned_state,
                diagnostics: Diagnostics::new(),
            }
        }

        async fn delete(&self, _: DeleteRequest) -> DeleteResponse {
            DeleteResponse {
                diagnostics: Diagnostics::new(),
            }
        }
    }

    let resource = Arc::new(PanicResource);

    // Test that panic in one task doesn't affect others
    let mut handles = vec![];

    // This one will panic
    let resource_clone = resource.clone();
    let panic_handle = tokio::spawn(async move {
        let mut config = Config::new();
        config
            .values
            .insert("should_panic".to_string(), Dynamic::Bool(true));

        resource_clone
            .create(CreateRequest {
                context: tfplug::context::Context::new(),
                config,
                planned_state: State::new(),
            })
            .await
    });
    handles.push(panic_handle);

    // These should succeed
    for _ in 0..3 {
        let resource_clone = resource.clone();
        let handle = tokio::spawn(async move {
            let config = Config::new();
            resource_clone
                .create(CreateRequest {
                    context: tfplug::context::Context::new(),
                    config,
                    planned_state: State::new(),
                })
                .await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    // One should have panicked
    let panicked = results.iter().filter(|r| r.is_err()).count();
    assert_eq!(panicked, 1);

    // Others should have succeeded
    let succeeded = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(succeeded, 3);
}

#[tokio::test]
async fn test_resource_cleanup_on_error() {
    struct CleanupResource {
        cleanup_count: Arc<RwLock<usize>>,
    }

    impl CleanupResource {
        fn new() -> Self {
            Self {
                cleanup_count: Arc::new(RwLock::new(0)),
            }
        }
    }

    #[async_trait]
    impl ResourceV2 for CleanupResource {
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

            // Simulate partial resource creation
            if request
                .config
                .get_bool("fail_after_partial")
                .unwrap_or(false)
            {
                // Increment cleanup counter
                let mut count = self.cleanup_count.write().await;
                *count += 1;

                diagnostics.add_error("Failed after partial creation", None::<String>);
                return CreateResponse {
                    state: request.planned_state,
                    diagnostics,
                };
            }

            CreateResponse {
                state: StateBuilder::new().string("id", "test").build(),
                diagnostics,
            }
        }

        async fn read(&self, request: ReadRequest) -> ReadResponse {
            ReadResponse {
                state: Some(request.current_state),
                diagnostics: Diagnostics::new(),
            }
        }

        async fn update(&self, request: UpdateRequest) -> UpdateResponse {
            UpdateResponse {
                state: request.planned_state,
                diagnostics: Diagnostics::new(),
            }
        }

        async fn delete(&self, _: DeleteRequest) -> DeleteResponse {
            let mut count = self.cleanup_count.write().await;
            *count = count.saturating_sub(1);

            DeleteResponse {
                diagnostics: Diagnostics::new(),
            }
        }
    }

    let resource = CleanupResource::new();
    let cleanup_count = resource.cleanup_count.clone();

    // Test failed creation
    let mut config = Config::new();
    config
        .values
        .insert("fail_after_partial".to_string(), Dynamic::Bool(true));

    let response = resource
        .create(CreateRequest {
            context: tfplug::context::Context::new(),
            config,
            planned_state: State::new(),
        })
        .await;

    assert!(!response.diagnostics.errors.is_empty());
    assert_eq!(*cleanup_count.read().await, 1);
}
