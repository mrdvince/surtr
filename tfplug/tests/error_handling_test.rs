//! Tests for error handling and edge cases in Provider async architecture

#![allow(clippy::disallowed_methods)] // Allow unwrap() in tests for clarity

use async_trait::async_trait;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;

// Import the new API modules
use tfplug::context::Context;
use tfplug::provider::{
    ConfigureProviderRequest, ConfigureProviderResponse, DataSourceFactory, Provider,
    ProviderMetaSchemaRequest, ProviderMetaSchemaResponse, ProviderMetadataRequest,
    ProviderMetadataResponse, ProviderSchemaRequest, ProviderSchemaResponse, ResourceFactory,
    StopProviderRequest, StopProviderResponse, ValidateProviderConfigRequest,
    ValidateProviderConfigResponse,
};
use tfplug::resource::{
    ConfigureResourceRequest, ConfigureResourceResponse, CreateResourceRequest,
    CreateResourceResponse, DeleteResourceRequest, DeleteResourceResponse, ReadResourceRequest,
    ReadResourceResponse, Resource, ResourceMetadataRequest, ResourceMetadataResponse,
    ResourceSchemaRequest, ResourceSchemaResponse, ResourceWithConfigure, UpdateResourceRequest,
    UpdateResourceResponse, ValidateResourceConfigRequest, ValidateResourceConfigResponse,
};
use tfplug::schema::{AttributeBuilder, AttributeType, SchemaBuilder};
use tfplug::types::{
    AttributePath, ClientCapabilities, Diagnostic, DiagnosticSeverity, DynamicValue,
    ServerCapabilities,
};

struct TimeoutProvider {
    operation_timeout: Duration,
}

impl TimeoutProvider {
    fn new(timeout_ms: u64) -> Self {
        Self {
            operation_timeout: Duration::from_millis(timeout_ms),
        }
    }
}

#[async_trait]
impl Provider for TimeoutProvider {
    fn type_name(&self) -> &str {
        "timeout"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse {
        ProviderMetadataResponse {
            type_name: "timeout".to_string(),
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
            schema: SchemaBuilder::new().build(),
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
        ConfigureProviderResponse {
            diagnostics: vec![],
            provider_data: Some(
                Arc::new(self.operation_timeout) as Arc<dyn std::any::Any + Send + Sync>
            ),
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
        let mut factories: HashMap<String, ResourceFactory> = HashMap::new();
        let timeout = self.operation_timeout;
        factories.insert(
            "timeout_resource".to_string(),
            Box::new(move || {
                Box::new(TimeoutResourceWithConfigure::new(timeout))
                    as Box<dyn ResourceWithConfigure>
            }),
        );
        factories
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
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
impl Resource for TimeoutResource {
    fn type_name(&self) -> &str {
        "timeout_resource"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: "timeout_resource".to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .version(1)
            .attribute(
                AttributeBuilder::new("delay_ms", AttributeType::Number)
                    .required()
                    .description("Operation delay in milliseconds")
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("id", AttributeType::String)
                    .computed()
                    .description("Resource ID")
                    .build(),
            )
            .build();

        ResourceSchemaResponse {
            schema,
            diagnostics: vec![],
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

    async fn create(
        &self,
        _ctx: Context,
        request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        let mut diagnostics = vec![];

        let delay_ms = match request.config.get_number(&AttributePath::new("delay_ms")) {
            Ok(d) => d as u64,
            Err(_) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "delay_ms is required".to_string(),
                    detail: "delay_ms attribute must be specified".to_string(),
                    attribute: Some(AttributePath::new("delay_ms")),
                });
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                };
            }
        };

        let operation = async {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            let mut state = request.config.clone();
            state
                .set_string(&AttributePath::new("id"), "test-id".to_string())
                .unwrap();
            state
        };

        match timeout(self.operation_timeout, operation).await {
            Ok(state) => CreateResourceResponse {
                new_state: state,
                private: vec![],
                diagnostics,
            },
            Err(_) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Operation timed out".to_string(),
                    detail: format!(
                        "Operation took longer than {} ms",
                        self.operation_timeout.as_millis()
                    ),
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
        ReadResourceResponse {
            new_state: Some(request.current_state),
            diagnostics: vec![],
            private: vec![],
            deferred: None,
            new_identity: None,
        }
    }

    async fn update(
        &self,
        _ctx: Context,
        request: UpdateResourceRequest,
    ) -> UpdateResourceResponse {
        UpdateResourceResponse {
            new_state: request.planned_state,
            private: vec![],
            diagnostics: vec![],
            new_identity: None,
        }
    }

    async fn delete(
        &self,
        _ctx: Context,
        _request: DeleteResourceRequest,
    ) -> DeleteResourceResponse {
        DeleteResourceResponse {
            diagnostics: vec![],
        }
    }
}

struct TimeoutResourceWithConfigure {
    resource: TimeoutResource,
    configured: bool,
}

impl TimeoutResourceWithConfigure {
    fn new(timeout: Duration) -> Self {
        Self {
            resource: TimeoutResource::new(timeout),
            configured: false,
        }
    }
}

#[async_trait]
impl Resource for TimeoutResourceWithConfigure {
    fn type_name(&self) -> &str {
        self.resource.type_name()
    }

    async fn metadata(
        &self,
        ctx: Context,
        request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        self.resource.metadata(ctx, request).await
    }

    async fn schema(&self, ctx: Context, request: ResourceSchemaRequest) -> ResourceSchemaResponse {
        self.resource.schema(ctx, request).await
    }

    async fn validate(
        &self,
        ctx: Context,
        request: ValidateResourceConfigRequest,
    ) -> ValidateResourceConfigResponse {
        self.resource.validate(ctx, request).await
    }

    async fn create(&self, ctx: Context, request: CreateResourceRequest) -> CreateResourceResponse {
        self.resource.create(ctx, request).await
    }

    async fn read(&self, ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
        self.resource.read(ctx, request).await
    }

    async fn update(&self, ctx: Context, request: UpdateResourceRequest) -> UpdateResourceResponse {
        self.resource.update(ctx, request).await
    }

    async fn delete(&self, ctx: Context, request: DeleteResourceRequest) -> DeleteResourceResponse {
        self.resource.delete(ctx, request).await
    }
}

#[async_trait]
impl ResourceWithConfigure for TimeoutResourceWithConfigure {
    async fn configure(
        &mut self,
        _ctx: Context,
        _request: ConfigureResourceRequest,
    ) -> ConfigureResourceResponse {
        self.configured = true;
        ConfigureResourceResponse {
            diagnostics: vec![],
        }
    }
}

struct RateLimitedProvider {
    semaphore: Arc<Semaphore>,
}

impl RateLimitedProvider {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
}

#[async_trait]
impl Provider for RateLimitedProvider {
    fn type_name(&self) -> &str {
        "rate_limited"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse {
        ProviderMetadataResponse {
            type_name: "rate_limited".to_string(),
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
            schema: SchemaBuilder::new().build(),
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
        ConfigureProviderResponse {
            diagnostics: vec![],
            provider_data: Some(self.semaphore.clone() as Arc<dyn std::any::Any + Send + Sync>),
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
        let mut factories: HashMap<String, ResourceFactory> = HashMap::new();
        let semaphore = self.semaphore.clone();
        factories.insert(
            "rate_limited".to_string(),
            Box::new(move || {
                Box::new(RateLimitedResourceWithConfigure::new(semaphore.clone()))
                    as Box<dyn ResourceWithConfigure>
            }),
        );
        factories
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
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
impl Resource for RateLimitedResource {
    fn type_name(&self) -> &str {
        "rate_limited"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: "rate_limited".to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .version(1)
            .attribute(
                AttributeBuilder::new("id", AttributeType::String)
                    .computed()
                    .description("Resource ID")
                    .build(),
            )
            .build();

        ResourceSchemaResponse {
            schema,
            diagnostics: vec![],
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

    async fn create(
        &self,
        _ctx: Context,
        request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        let mut diagnostics = vec![];

        // Acquire semaphore permit
        let _permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed to acquire rate limit permit".to_string(),
                    detail: "Unable to acquire semaphore permit for rate limiting".to_string(),
                    attribute: None,
                });
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                };
            }
        };

        // Simulate work
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut state = request.config.clone();
        state
            .set_string(
                &AttributePath::new("id"),
                format!("rate-limited-{}", uuid::Uuid::new_v4()),
            )
            .unwrap();

        CreateResourceResponse {
            new_state: state,
            private: vec![],
            diagnostics,
        }
    }

    async fn read(&self, _ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
        ReadResourceResponse {
            new_state: Some(request.current_state),
            diagnostics: vec![],
            private: vec![],
            deferred: None,
            new_identity: None,
        }
    }

    async fn update(
        &self,
        _ctx: Context,
        request: UpdateResourceRequest,
    ) -> UpdateResourceResponse {
        UpdateResourceResponse {
            new_state: request.planned_state,
            private: vec![],
            diagnostics: vec![],
            new_identity: None,
        }
    }

    async fn delete(
        &self,
        _ctx: Context,
        _request: DeleteResourceRequest,
    ) -> DeleteResourceResponse {
        DeleteResourceResponse {
            diagnostics: vec![],
        }
    }
}

struct RateLimitedResourceWithConfigure {
    resource: RateLimitedResource,
    configured: bool,
}

impl RateLimitedResourceWithConfigure {
    fn new(semaphore: Arc<Semaphore>) -> Self {
        Self {
            resource: RateLimitedResource::new(semaphore),
            configured: false,
        }
    }
}

#[async_trait]
impl Resource for RateLimitedResourceWithConfigure {
    fn type_name(&self) -> &str {
        self.resource.type_name()
    }

    async fn metadata(
        &self,
        ctx: Context,
        request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        self.resource.metadata(ctx, request).await
    }

    async fn schema(&self, ctx: Context, request: ResourceSchemaRequest) -> ResourceSchemaResponse {
        self.resource.schema(ctx, request).await
    }

    async fn validate(
        &self,
        ctx: Context,
        request: ValidateResourceConfigRequest,
    ) -> ValidateResourceConfigResponse {
        self.resource.validate(ctx, request).await
    }

    async fn create(&self, ctx: Context, request: CreateResourceRequest) -> CreateResourceResponse {
        self.resource.create(ctx, request).await
    }

    async fn read(&self, ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
        self.resource.read(ctx, request).await
    }

    async fn update(&self, ctx: Context, request: UpdateResourceRequest) -> UpdateResourceResponse {
        self.resource.update(ctx, request).await
    }

    async fn delete(&self, ctx: Context, request: DeleteResourceRequest) -> DeleteResourceResponse {
        self.resource.delete(ctx, request).await
    }
}

#[async_trait]
impl ResourceWithConfigure for RateLimitedResourceWithConfigure {
    async fn configure(
        &mut self,
        _ctx: Context,
        _request: ConfigureResourceRequest,
    ) -> ConfigureResourceResponse {
        self.configured = true;
        ConfigureResourceResponse {
            diagnostics: vec![],
        }
    }
}

#[tokio::test]
async fn test_operation_timeout() {
    let mut provider = TimeoutProvider::new(100); // 100ms timeout
    provider
        .configure(
            Context::new(),
            ConfigureProviderRequest {
                terraform_version: "1.0.0".to_string(),
                config: DynamicValue::null(),
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    let resources = provider.resources();
    let resource_factory = resources.get("timeout_resource").unwrap();
    let resource = resource_factory();

    // Test operation that completes within timeout
    let mut config = DynamicValue::null();
    config
        .set_number(&AttributePath::new("delay_ms"), 50.0)
        .unwrap(); // 50ms delay

    let response = resource
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "timeout_resource".to_string(),
                config: config.clone(),
                planned_state: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;

    assert!(response.diagnostics.is_empty());
    assert!(response
        .new_state
        .get_string(&AttributePath::new("id"))
        .is_ok());

    // Test operation that exceeds timeout
    config
        .set_number(&AttributePath::new("delay_ms"), 200.0)
        .unwrap(); // 200ms delay

    let response = resource
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "timeout_resource".to_string(),
                config,
                planned_state: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;

    assert!(!response.diagnostics.is_empty());
    assert!(response.diagnostics[0].summary.contains("timed out"));
}

#[tokio::test]
async fn test_rate_limiting() {
    let mut provider = RateLimitedProvider::new(2); // Allow 2 concurrent operations
    provider
        .configure(
            Context::new(),
            ConfigureProviderRequest {
                terraform_version: "1.0.0".to_string(),
                config: DynamicValue::null(),
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    let resources = provider.resources();
    let resource_factory = resources.get("rate_limited").unwrap();

    // Start 4 concurrent operations (only 2 should run at a time)
    let start = tokio::time::Instant::now();
    let mut handles = vec![];

    for i in 0..4 {
        let resource = resource_factory();
        let handle = tokio::spawn(async move {
            let config = DynamicValue::null();
            let response = resource
                .create(
                    Context::new(),
                    CreateResourceRequest {
                        type_name: "rate_limited".to_string(),
                        config,
                        planned_state: DynamicValue::null(),
                        planned_private: vec![],
                        provider_meta: None,
                    },
                )
                .await;

            assert!(response.diagnostics.is_empty());
            (i, tokio::time::Instant::now())
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;
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
    impl Resource for PanicResource {
        fn type_name(&self) -> &str {
            "panic"
        }

        async fn metadata(
            &self,
            _ctx: Context,
            _request: ResourceMetadataRequest,
        ) -> ResourceMetadataResponse {
            ResourceMetadataResponse {
                type_name: "panic".to_string(),
            }
        }

        async fn schema(
            &self,
            _ctx: Context,
            _request: ResourceSchemaRequest,
        ) -> ResourceSchemaResponse {
            let schema = SchemaBuilder::new()
                .version(1)
                .attribute(
                    AttributeBuilder::new("should_panic", AttributeType::Bool)
                        .optional()
                        .description("Whether to panic")
                        .build(),
                )
                .build();

            ResourceSchemaResponse {
                schema,
                diagnostics: vec![],
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

        async fn create(
            &self,
            _ctx: Context,
            request: CreateResourceRequest,
        ) -> CreateResourceResponse {
            if request
                .config
                .get_bool(&AttributePath::new("should_panic"))
                .unwrap_or(false)
            {
                panic!("Intentional panic for testing");
            }

            CreateResourceResponse {
                new_state: DynamicValue::null(),
                private: vec![],
                diagnostics: vec![],
            }
        }

        async fn read(&self, _ctx: Context, _request: ReadResourceRequest) -> ReadResourceResponse {
            ReadResourceResponse {
                new_state: Some(DynamicValue::null()),
                diagnostics: vec![],
                private: vec![],
                deferred: None,
                new_identity: None,
            }
        }

        async fn update(
            &self,
            _ctx: Context,
            request: UpdateResourceRequest,
        ) -> UpdateResourceResponse {
            UpdateResourceResponse {
                new_state: request.planned_state,
                private: vec![],
                diagnostics: vec![],
                new_identity: None,
            }
        }

        async fn delete(
            &self,
            _ctx: Context,
            _request: DeleteResourceRequest,
        ) -> DeleteResourceResponse {
            DeleteResourceResponse {
                diagnostics: vec![],
            }
        }
    }

    let resource = Arc::new(PanicResource);

    // Test that panic in one task doesn't affect others
    let mut handles = vec![];

    // This one will panic
    let resource_clone = resource.clone();
    let panic_handle = tokio::spawn(async move {
        let mut config = DynamicValue::null();
        config
            .set_bool(&AttributePath::new("should_panic"), true)
            .unwrap();

        resource_clone
            .create(
                Context::new(),
                CreateResourceRequest {
                    type_name: "panic".to_string(),
                    config,
                    planned_state: DynamicValue::null(),
                    planned_private: vec![],
                    provider_meta: None,
                },
            )
            .await
    });
    handles.push(panic_handle);

    // These should succeed
    for _ in 0..3 {
        let resource_clone = resource.clone();
        let handle = tokio::spawn(async move {
            let config = DynamicValue::null();
            resource_clone
                .create(
                    Context::new(),
                    CreateResourceRequest {
                        type_name: "panic".to_string(),
                        config,
                        planned_state: DynamicValue::null(),
                        planned_private: vec![],
                        provider_meta: None,
                    },
                )
                .await
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;

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
    impl Resource for CleanupResource {
        fn type_name(&self) -> &str {
            "cleanup"
        }

        async fn metadata(
            &self,
            _ctx: Context,
            _request: ResourceMetadataRequest,
        ) -> ResourceMetadataResponse {
            ResourceMetadataResponse {
                type_name: "cleanup".to_string(),
            }
        }

        async fn schema(
            &self,
            _ctx: Context,
            _request: ResourceSchemaRequest,
        ) -> ResourceSchemaResponse {
            let schema = SchemaBuilder::new()
                .version(1)
                .attribute(
                    AttributeBuilder::new("fail_after_partial", AttributeType::Bool)
                        .optional()
                        .description("Whether to fail after partial creation")
                        .build(),
                )
                .attribute(
                    AttributeBuilder::new("id", AttributeType::String)
                        .computed()
                        .description("Resource ID")
                        .build(),
                )
                .build();

            ResourceSchemaResponse {
                schema,
                diagnostics: vec![],
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

        async fn create(
            &self,
            _ctx: Context,
            request: CreateResourceRequest,
        ) -> CreateResourceResponse {
            let mut diagnostics = vec![];

            // Simulate partial resource creation
            if request
                .config
                .get_bool(&AttributePath::new("fail_after_partial"))
                .unwrap_or(false)
            {
                // Increment cleanup counter
                let mut count = self.cleanup_count.write().await;
                *count += 1;

                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    summary: "Failed after partial creation".to_string(),
                    detail: "Simulated partial creation failure".to_string(),
                    attribute: None,
                });
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                };
            }

            let mut state = request.config.clone();
            state
                .set_string(&AttributePath::new("id"), "test".to_string())
                .unwrap();

            CreateResourceResponse {
                new_state: state,
                private: vec![],
                diagnostics,
            }
        }

        async fn read(&self, _ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
            ReadResourceResponse {
                new_state: Some(request.current_state),
                diagnostics: vec![],
                private: vec![],
                deferred: None,
                new_identity: None,
            }
        }

        async fn update(
            &self,
            _ctx: Context,
            request: UpdateResourceRequest,
        ) -> UpdateResourceResponse {
            UpdateResourceResponse {
                new_state: request.planned_state,
                private: vec![],
                diagnostics: vec![],
                new_identity: None,
            }
        }

        async fn delete(
            &self,
            _ctx: Context,
            _request: DeleteResourceRequest,
        ) -> DeleteResourceResponse {
            let mut count = self.cleanup_count.write().await;
            *count = count.saturating_sub(1);

            DeleteResourceResponse {
                diagnostics: vec![],
            }
        }
    }

    let resource = CleanupResource::new();
    let cleanup_count = resource.cleanup_count.clone();

    // Test failed creation
    let mut config = DynamicValue::null();
    config
        .set_bool(&AttributePath::new("fail_after_partial"), true)
        .unwrap();

    let response = resource
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "cleanup".to_string(),
                config,
                planned_state: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;

    assert!(!response.diagnostics.is_empty());
    assert_eq!(*cleanup_count.read().await, 1);
}
