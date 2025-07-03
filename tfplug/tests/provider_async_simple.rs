//! Simple test to verify async traits work correctly in provider

#![allow(clippy::disallowed_methods)] // Allow unwrap() in tests for clarity

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;
use tokio::time::sleep;

// Import the async traits from new API
use tfplug::context::Context;
use tfplug::data_source::{
    ConfigureDataSourceRequest, ConfigureDataSourceResponse, DataSource, DataSourceMetadataRequest,
    DataSourceMetadataResponse, DataSourceSchemaRequest, DataSourceSchemaResponse,
    DataSourceWithConfigure, ReadDataSourceRequest, ReadDataSourceResponse,
    ValidateDataSourceConfigRequest, ValidateDataSourceConfigResponse,
};
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
use tfplug::schema::SchemaBuilder;
use tfplug::types::{ClientCapabilities, DynamicValue, ServerCapabilities};

// Simple provider implementation
struct SimpleProvider;

#[async_trait]
impl Provider for SimpleProvider {
    fn type_name(&self) -> &str {
        "simple"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse {
        ProviderMetadataResponse {
            type_name: "simple".to_string(),
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
        // Simulate async work
        sleep(Duration::from_millis(1)).await;
        ConfigureProviderResponse {
            diagnostics: vec![],
            provider_data: Some(
                Arc::new("configured".to_string()) as Arc<dyn std::any::Any + Send + Sync>
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
        factories.insert(
            "simple".to_string(),
            Box::new(|| {
                Box::new(SimpleResourceWithConfigure::new()) as Box<dyn ResourceWithConfigure>
            }),
        );
        factories.insert(
            "slow".to_string(),
            Box::new(|| {
                Box::new(SlowResourceWithConfigure::new()) as Box<dyn ResourceWithConfigure>
            }),
        );
        factories
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
        let mut factories: HashMap<String, DataSourceFactory> = HashMap::new();
        factories.insert(
            "simple".to_string(),
            Box::new(|| {
                Box::new(SimpleDataSourceWithConfigure::new()) as Box<dyn DataSourceWithConfigure>
            }),
        );
        factories
    }
}

struct SimpleResource;

impl SimpleResource {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Resource for SimpleResource {
    fn type_name(&self) -> &str {
        "simple"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: "simple".to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        ResourceSchemaResponse {
            schema: SchemaBuilder::new().build(),
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
        _request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        // Simulate async work
        sleep(Duration::from_millis(1)).await;
        CreateResourceResponse {
            new_state: DynamicValue::null(),
            private: vec![],
            diagnostics: vec![],
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
        _request: UpdateResourceRequest,
    ) -> UpdateResourceResponse {
        UpdateResourceResponse {
            new_state: DynamicValue::null(),
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

struct SimpleResourceWithConfigure {
    resource: SimpleResource,
    configured: bool,
}

impl SimpleResourceWithConfigure {
    fn new() -> Self {
        Self {
            resource: SimpleResource::new(),
            configured: false,
        }
    }
}

#[async_trait]
impl Resource for SimpleResourceWithConfigure {
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
impl ResourceWithConfigure for SimpleResourceWithConfigure {
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

struct SlowResource;

impl SlowResource {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Resource for SlowResource {
    fn type_name(&self) -> &str {
        "slow"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: "slow".to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        ResourceSchemaResponse {
            schema: SchemaBuilder::new().build(),
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
        _request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        // Simulate slow async operation
        sleep(Duration::from_millis(100)).await;
        CreateResourceResponse {
            new_state: DynamicValue::null(),
            private: vec![],
            diagnostics: vec![],
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
        _request: UpdateResourceRequest,
    ) -> UpdateResourceResponse {
        UpdateResourceResponse {
            new_state: DynamicValue::null(),
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

struct SlowResourceWithConfigure {
    resource: SlowResource,
    configured: bool,
}

impl SlowResourceWithConfigure {
    fn new() -> Self {
        Self {
            resource: SlowResource::new(),
            configured: false,
        }
    }
}

#[async_trait]
impl Resource for SlowResourceWithConfigure {
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
impl ResourceWithConfigure for SlowResourceWithConfigure {
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

struct SimpleDataSource;

impl SimpleDataSource {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DataSource for SimpleDataSource {
    fn type_name(&self) -> &str {
        "simple"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: DataSourceMetadataRequest,
    ) -> DataSourceMetadataResponse {
        DataSourceMetadataResponse {
            type_name: "simple".to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: DataSourceSchemaRequest,
    ) -> DataSourceSchemaResponse {
        DataSourceSchemaResponse {
            schema: SchemaBuilder::new().build(),
            diagnostics: vec![],
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

    async fn read(&self, _ctx: Context, _request: ReadDataSourceRequest) -> ReadDataSourceResponse {
        ReadDataSourceResponse {
            state: DynamicValue::null(),
            diagnostics: vec![],
            deferred: None,
        }
    }
}

struct SimpleDataSourceWithConfigure {
    data_source: SimpleDataSource,
    configured: bool,
}

impl SimpleDataSourceWithConfigure {
    fn new() -> Self {
        Self {
            data_source: SimpleDataSource::new(),
            configured: false,
        }
    }
}

#[async_trait]
impl DataSource for SimpleDataSourceWithConfigure {
    fn type_name(&self) -> &str {
        self.data_source.type_name()
    }

    async fn metadata(
        &self,
        ctx: Context,
        request: DataSourceMetadataRequest,
    ) -> DataSourceMetadataResponse {
        self.data_source.metadata(ctx, request).await
    }

    async fn schema(
        &self,
        ctx: Context,
        request: DataSourceSchemaRequest,
    ) -> DataSourceSchemaResponse {
        self.data_source.schema(ctx, request).await
    }

    async fn validate(
        &self,
        ctx: Context,
        request: ValidateDataSourceConfigRequest,
    ) -> ValidateDataSourceConfigResponse {
        self.data_source.validate(ctx, request).await
    }

    async fn read(&self, ctx: Context, request: ReadDataSourceRequest) -> ReadDataSourceResponse {
        self.data_source.read(ctx, request).await
    }
}

#[async_trait]
impl DataSourceWithConfigure for SimpleDataSourceWithConfigure {
    async fn configure(
        &mut self,
        _ctx: Context,
        _request: ConfigureDataSourceRequest,
    ) -> ConfigureDataSourceResponse {
        self.configured = true;
        ConfigureDataSourceResponse {
            diagnostics: vec![],
        }
    }
}

#[tokio::test]
async fn test_async_trait_methods_work() {
    let mut provider = SimpleProvider;

    // Test configure
    let config_resp = provider
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
    assert_eq!(config_resp.diagnostics.len(), 0);

    // Test factory methods
    let resources = provider.resources();
    let resource_factory = resources.get("simple").unwrap();
    let resource = resource_factory();

    let data_sources = provider.data_sources();
    let data_source_factory = data_sources.get("simple").unwrap();
    let data_source = data_source_factory();

    // Test resource methods
    let create_resp = resource
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "simple".to_string(),
                planned_state: DynamicValue::null(),
                config: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;
    assert_eq!(create_resp.diagnostics.len(), 0);

    // Test data source methods
    let read_resp = data_source
        .read(
            Context::new(),
            ReadDataSourceRequest {
                type_name: "simple".to_string(),
                config: DynamicValue::null(),
                provider_meta: None,
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;
    assert_eq!(read_resp.diagnostics.len(), 0);
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
            let resources = provider_clone.resources();
            let resource_factory = resources.get("slow").unwrap();
            let resource = resource_factory();
            let _resp = resource
                .create(
                    Context::new(),
                    CreateResourceRequest {
                        type_name: "slow".to_string(),
                        planned_state: DynamicValue::null(),
                        config: DynamicValue::null(),
                        planned_private: vec![],
                        provider_meta: None,
                    },
                )
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
    let resources = provider.resources();
    let result = resources.get("unknown");
    assert!(result.is_none());

    // Test unknown data source error
    let data_sources = provider.data_sources();
    let result = data_sources.get("unknown");
    assert!(result.is_none());

    // Test that known resources and data sources work
    let resource_factory = resources.get("simple");
    assert!(resource_factory.is_some());

    let data_source_factory = data_sources.get("simple");
    assert!(data_source_factory.is_some());
}

#[tokio::test]
async fn test_factory_creates_new_instances() {
    let provider = SimpleProvider;

    // Get factory
    let resources = provider.resources();
    let resource_factory = resources.get("simple").unwrap();

    // Create multiple instances
    let resource1 = resource_factory();
    let resource2 = resource_factory();

    // Both should succeed
    let resp1 = resource1
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "simple".to_string(),
                planned_state: DynamicValue::null(),
                config: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;

    let resp2 = resource2
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "simple".to_string(),
                planned_state: DynamicValue::null(),
                config: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;

    assert_eq!(resp1.diagnostics.len(), 0);
    assert_eq!(resp2.diagnostics.len(), 0);
}
