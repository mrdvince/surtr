#![allow(clippy::disallowed_methods)] // Allow unwrap() in tests for clarity

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task;

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

struct TestProvider;

#[async_trait]
impl Provider for TestProvider {
    fn type_name(&self) -> &str {
        "test"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse {
        ProviderMetadataResponse {
            type_name: "test".to_string(),
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
            "test".to_string(),
            Box::new(|| {
                Box::new(TestResourceWithConfigure::new()) as Box<dyn ResourceWithConfigure>
            }),
        );
        factories
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
        let mut factories: HashMap<String, DataSourceFactory> = HashMap::new();
        factories.insert(
            "test".to_string(),
            Box::new(|| {
                Box::new(TestDataSourceWithConfigure::new()) as Box<dyn DataSourceWithConfigure>
            }),
        );
        factories
    }
}

struct TestResource;

impl TestResource {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Resource for TestResource {
    fn type_name(&self) -> &str {
        "test"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: "test".to_string(),
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

struct TestResourceWithConfigure {
    resource: TestResource,
    configured: bool,
}

impl TestResourceWithConfigure {
    fn new() -> Self {
        Self {
            resource: TestResource::new(),
            configured: false,
        }
    }
}

#[async_trait]
impl Resource for TestResourceWithConfigure {
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
impl ResourceWithConfigure for TestResourceWithConfigure {
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

struct TestDataSource;

impl TestDataSource {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DataSource for TestDataSource {
    fn type_name(&self) -> &str {
        "test"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: DataSourceMetadataRequest,
    ) -> DataSourceMetadataResponse {
        DataSourceMetadataResponse {
            type_name: "test".to_string(),
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

struct TestDataSourceWithConfigure {
    data_source: TestDataSource,
    configured: bool,
}

impl TestDataSourceWithConfigure {
    fn new() -> Self {
        Self {
            data_source: TestDataSource::new(),
            configured: false,
        }
    }
}

#[async_trait]
impl DataSource for TestDataSourceWithConfigure {
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
impl DataSourceWithConfigure for TestDataSourceWithConfigure {
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
async fn test_async_provider_works() {
    let mut provider = TestProvider;

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
    let resource_factory = resources.get("test").unwrap();
    let resource = resource_factory();

    let create_resp = resource
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "test".to_string(),
                planned_state: DynamicValue::null(),
                config: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;

    assert_eq!(create_resp.diagnostics.len(), 0);
}

#[tokio::test]
async fn test_concurrent_resource_creation() {
    let provider = Arc::new(TestProvider);
    let mut handles = vec![];

    for _ in 0..5 {
        let provider_clone = provider.clone();
        let handle = task::spawn(async move {
            let resources = provider_clone.resources();
            let resource_factory = resources.get("test").unwrap();
            let resource = resource_factory();
            let _resp = resource
                .create(
                    Context::new(),
                    CreateResourceRequest {
                        type_name: "test".to_string(),
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

    for handle in handles {
        handle.await.unwrap();
    }
}
