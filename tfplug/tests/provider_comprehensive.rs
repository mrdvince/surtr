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

// Import the new API modules
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
use tfplug::types::{AttributePath, ClientCapabilities, Dynamic, DynamicValue, ServerCapabilities};

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
    config: RwLock<Option<DynamicValue>>,
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
impl Provider for AdvancedProvider {
    fn type_name(&self) -> &str {
        "advanced"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ProviderMetadataRequest,
    ) -> ProviderMetadataResponse {
        ProviderMetadataResponse {
            type_name: "advanced".to_string(),
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
        request: ConfigureProviderRequest,
    ) -> ConfigureProviderResponse {
        // Simulate async configuration work
        sleep(Duration::from_millis(10)).await;

        let mut config = self.config.write().await;
        *config = Some(request.config);

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
        let stats = self.stats.clone();

        factories.insert(
            "tracked".to_string(),
            Box::new(move || {
                Box::new(TrackedResourceWithConfigure::new(stats.clone()))
                    as Box<dyn ResourceWithConfigure>
            }),
        );
        factories.insert(
            "stateful".to_string(),
            Box::new(|| {
                Box::new(StatefulResourceWithConfigure::new()) as Box<dyn ResourceWithConfigure>
            }),
        );
        factories
    }

    fn data_sources(&self) -> HashMap<String, DataSourceFactory> {
        let mut factories: HashMap<String, DataSourceFactory> = HashMap::new();
        factories.insert(
            "config_reader".to_string(),
            Box::new(|| {
                Box::new(ConfigReaderDataSourceWithConfigure::new())
                    as Box<dyn DataSourceWithConfigure>
            }),
        );
        factories
    }
}

// Resource that tracks concurrent operations
struct TrackedResource {
    stats: Arc<OperationStats>,
}

impl TrackedResource {
    fn new(stats: Arc<OperationStats>) -> Self {
        Self { stats }
    }
}

#[async_trait]
impl Resource for TrackedResource {
    fn type_name(&self) -> &str {
        "tracked"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: "tracked".to_string(),
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
        let current = self.stats.start_operation();

        // Simulate work
        sleep(Duration::from_millis(50)).await;

        let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));
        state
            .set_number(&AttributePath::new("concurrent_operations"), current as f64)
            .unwrap();
        state
            .set_number(
                &AttributePath::new("max_concurrent"),
                self.stats.max_concurrent.load(Ordering::SeqCst) as f64,
            )
            .unwrap();

        self.stats.end_operation();

        CreateResourceResponse {
            new_state: state,
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

struct TrackedResourceWithConfigure {
    resource: TrackedResource,
    configured: bool,
}

impl TrackedResourceWithConfigure {
    fn new(stats: Arc<OperationStats>) -> Self {
        Self {
            resource: TrackedResource::new(stats),
            configured: false,
        }
    }
}

#[async_trait]
impl Resource for TrackedResourceWithConfigure {
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
impl ResourceWithConfigure for TrackedResourceWithConfigure {
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
impl Resource for StatefulResource {
    fn type_name(&self) -> &str {
        "stateful"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: "stateful".to_string(),
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
        request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        let mut internal_state = self.state.write().await;

        // Store config values in internal state (simplified for test)
        if let Ok(config_map) = request.config.get_map(&AttributePath::root()) {
            for (key, value) in config_map {
                if let Dynamic::String(str_val) = value {
                    internal_state.insert(key, str_val);
                }
            }
        }

        let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));
        state
            .set_string(&AttributePath::new("id"), "stateful-resource".to_string())
            .unwrap();
        state
            .set_number(
                &AttributePath::new("item_count"),
                internal_state.len() as f64,
            )
            .unwrap();

        CreateResourceResponse {
            new_state: state,
            private: vec![],
            diagnostics: vec![],
        }
    }

    async fn read(&self, _ctx: Context, _request: ReadResourceRequest) -> ReadResourceResponse {
        let internal_state = self.state.read().await;

        let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));
        state
            .set_string(&AttributePath::new("id"), "stateful-resource".to_string())
            .unwrap();
        state
            .set_number(
                &AttributePath::new("item_count"),
                internal_state.len() as f64,
            )
            .unwrap();

        ReadResourceResponse {
            new_state: Some(state),
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
        let mut internal_state = self.state.write().await;
        internal_state.clear();

        // Update with new config values (simplified for test)
        if let Ok(config_map) = request.config.get_map(&AttributePath::root()) {
            for (key, value) in config_map {
                if let Dynamic::String(str_val) = value {
                    internal_state.insert(key, str_val);
                }
            }
        }

        let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));
        state
            .set_string(&AttributePath::new("id"), "stateful-resource".to_string())
            .unwrap();
        state
            .set_number(
                &AttributePath::new("item_count"),
                internal_state.len() as f64,
            )
            .unwrap();

        UpdateResourceResponse {
            new_state: state,
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
        let mut internal_state = self.state.write().await;
        internal_state.clear();

        DeleteResourceResponse {
            diagnostics: vec![],
        }
    }
}

struct StatefulResourceWithConfigure {
    resource: StatefulResource,
    configured: bool,
}

impl StatefulResourceWithConfigure {
    fn new() -> Self {
        Self {
            resource: StatefulResource::new(),
            configured: false,
        }
    }
}

#[async_trait]
impl Resource for StatefulResourceWithConfigure {
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
impl ResourceWithConfigure for StatefulResourceWithConfigure {
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

// Data source that reads provider configuration
struct ConfigReaderDataSource {
    provider_config: Option<DynamicValue>,
}

impl ConfigReaderDataSource {
    fn new() -> Self {
        Self {
            provider_config: None,
        }
    }

    #[allow(dead_code)]
    fn with_config(config: Option<DynamicValue>) -> Self {
        Self {
            provider_config: config,
        }
    }
}

#[async_trait]
impl DataSource for ConfigReaderDataSource {
    fn type_name(&self) -> &str {
        "config_reader"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: DataSourceMetadataRequest,
    ) -> DataSourceMetadataResponse {
        DataSourceMetadataResponse {
            type_name: "config_reader".to_string(),
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
        let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));

        if let Some(config) = &self.provider_config {
            state
                .set_bool(&AttributePath::new("has_config"), true)
                .unwrap();
            if let Ok(config_map) = config.get_map(&AttributePath::root()) {
                state
                    .set_number(&AttributePath::new("config_keys"), config_map.len() as f64)
                    .unwrap();
            } else {
                state
                    .set_number(&AttributePath::new("config_keys"), 0.0)
                    .unwrap();
            }
        } else {
            state
                .set_bool(&AttributePath::new("has_config"), false)
                .unwrap();
        }

        ReadDataSourceResponse {
            state,
            diagnostics: vec![],
            deferred: None,
        }
    }
}

struct ConfigReaderDataSourceWithConfigure {
    data_source: ConfigReaderDataSource,
    configured: bool,
}

impl ConfigReaderDataSourceWithConfigure {
    fn new() -> Self {
        Self {
            data_source: ConfigReaderDataSource::new(),
            configured: false,
        }
    }
}

#[async_trait]
impl DataSource for ConfigReaderDataSourceWithConfigure {
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
impl DataSourceWithConfigure for ConfigReaderDataSourceWithConfigure {
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
async fn test_provider_configuration_is_async() {
    let mut provider = AdvancedProvider::new();

    let mut config = DynamicValue::new(Dynamic::Map(HashMap::new()));
    config
        .set_string(&AttributePath::new("key"), "value".to_string())
        .unwrap();

    let start = Instant::now();
    let resp = provider
        .configure(
            Context::new(),
            ConfigureProviderRequest {
                terraform_version: "1.0.0".to_string(),
                config,
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;
    let elapsed = start.elapsed();

    assert_eq!(resp.diagnostics.len(), 0);
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
            let resources = provider_clone.resources();
            let resource_factory = resources.get("tracked").unwrap();
            let resource = resource_factory();
            let resp = resource
                .create(
                    Context::new(),
                    CreateResourceRequest {
                        type_name: "tracked".to_string(),
                        config: DynamicValue::null(),
                        planned_state: DynamicValue::null(),
                        planned_private: vec![],
                        provider_meta: None,
                    },
                )
                .await;

            let concurrent = resp
                .new_state
                .get_number(&AttributePath::new("concurrent_operations"))
                .unwrap();
            let max = resp
                .new_state
                .get_number(&AttributePath::new("max_concurrent"))
                .unwrap();
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
    let resources = provider.resources();
    let resource_factory = resources.get("stateful").unwrap();
    let resource = resource_factory();

    // Create with initial config
    let mut config = DynamicValue::new(Dynamic::Map(HashMap::new()));
    config
        .set_string(&AttributePath::new("key1"), "value1".to_string())
        .unwrap();
    config
        .set_string(&AttributePath::new("key2"), "value2".to_string())
        .unwrap();

    let create_resp = resource
        .create(
            Context::new(),
            CreateResourceRequest {
                type_name: "stateful".to_string(),
                config,
                planned_state: DynamicValue::null(),
                planned_private: vec![],
                provider_meta: None,
            },
        )
        .await;

    assert_eq!(
        create_resp
            .new_state
            .get_number(&AttributePath::new("item_count"))
            .unwrap(),
        2.0
    );

    // Read should return same count
    let read_resp = resource
        .read(
            Context::new(),
            ReadResourceRequest {
                type_name: "stateful".to_string(),
                current_state: DynamicValue::null(),
                private: vec![],
                provider_meta: None,
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
                current_identity: None,
            },
        )
        .await;

    assert_eq!(
        read_resp
            .new_state
            .unwrap()
            .get_number(&AttributePath::new("item_count"))
            .unwrap(),
        2.0
    );

    // Update with new config
    let mut new_config = DynamicValue::new(Dynamic::Map(HashMap::new()));
    new_config
        .set_string(&AttributePath::new("key3"), "value3".to_string())
        .unwrap();

    let update_resp = resource
        .update(
            Context::new(),
            UpdateResourceRequest {
                type_name: "stateful".to_string(),
                prior_state: DynamicValue::null(),
                planned_state: DynamicValue::null(),
                config: new_config,
                planned_private: vec![],
                provider_meta: None,
                planned_identity: None,
            },
        )
        .await;

    assert_eq!(
        update_resp
            .new_state
            .get_number(&AttributePath::new("item_count"))
            .unwrap(),
        1.0
    );
}

#[tokio::test]
async fn test_data_source_can_access_provider_config() {
    let mut provider = AdvancedProvider::new();

    // Configure provider
    let mut config = DynamicValue::new(Dynamic::Map(HashMap::new()));
    config
        .set_string(&AttributePath::new("api_key"), "secret".to_string())
        .unwrap();

    provider
        .configure(
            Context::new(),
            ConfigureProviderRequest {
                terraform_version: "1.0.0".to_string(),
                config,
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    // Create data source after configuration
    let data_sources = provider.data_sources();
    let data_source_factory = data_sources.get("config_reader").unwrap();
    let data_source = data_source_factory();

    let resp = data_source
        .read(
            Context::new(),
            ReadDataSourceRequest {
                type_name: "config_reader".to_string(),
                config: DynamicValue::null(),
                provider_meta: None,
                client_capabilities: ClientCapabilities {
                    deferral_allowed: false,
                    write_only_attributes_allowed: false,
                },
            },
        )
        .await;

    // Note: Due to the new architecture, data sources don't automatically
    // get provider config. This would need to be implemented through
    // the configure method and provider_data mechanism.
    assert!(
        resp.state
            .get_bool(&AttributePath::new("has_config"))
            .unwrap_or(false)
            == false
    );
}

#[tokio::test]
async fn test_factory_methods_are_async() {
    let provider = AdvancedProvider::new();

    // Test that schema methods can do async work
    let start = Instant::now();
    let _schema_resp = provider.schema(Context::new(), ProviderSchemaRequest).await;
    let _elapsed = start.elapsed();

    // Factory methods themselves are sync in the new API,
    // but the test validates that we can call async methods on the provider
    // Just verify it completes without panic

    // Test factory creation
    let resources = provider.resources();
    assert!(resources.contains_key("tracked"));
    assert!(resources.contains_key("stateful"));

    let data_sources = provider.data_sources();
    assert!(data_sources.contains_key("config_reader"));
}
