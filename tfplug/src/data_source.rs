//! DataSource trait and related types
//!
//! This module defines the DataSource trait that data sources must implement.

use crate::context::Context;
use crate::schema::Schema;
use crate::types::{ClientCapabilities, Deferred, Diagnostic, DynamicValue};
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

/// Base trait for data sources - implement read operations
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Type name should be constant (e.g., "proxmox_version")
    /// MUST match the key used in Provider.data_sources()
    fn type_name(&self) -> &str;

    /// Called to get data source metadata
    async fn metadata(
        &self,
        ctx: Context,
        request: DataSourceMetadataRequest,
    ) -> DataSourceMetadataResponse;

    /// Called to get data source schema - cache this in your implementation
    async fn schema(
        &self,
        ctx: Context,
        request: DataSourceSchemaRequest,
    ) -> DataSourceSchemaResponse;

    /// Called during plan to validate configuration
    async fn validate(
        &self,
        ctx: Context,
        request: ValidateDataSourceConfigRequest,
    ) -> ValidateDataSourceConfigResponse;

    /// Called to read data - this is the only operation for data sources
    /// MUST populate all attributes in response.state
    async fn read(&self, ctx: Context, request: ReadDataSourceRequest) -> ReadDataSourceResponse;
}

// Request/Response Types
pub struct DataSourceMetadataRequest;

pub struct DataSourceMetadataResponse {
    pub type_name: String,
}

pub struct DataSourceSchemaRequest;

pub struct DataSourceSchemaResponse {
    pub schema: Schema,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ValidateDataSourceConfigRequest {
    pub type_name: String,
    pub config: DynamicValue,
}

pub struct ValidateDataSourceConfigResponse {
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ReadDataSourceRequest {
    pub type_name: String,
    pub config: DynamicValue,
    pub provider_meta: Option<DynamicValue>,
    pub client_capabilities: ClientCapabilities,
}

pub struct ReadDataSourceResponse {
    pub state: DynamicValue,
    pub diagnostics: Vec<Diagnostic>,
    pub deferred: Option<Deferred>,
}

/// All data sources must implement configure to receive provider data
/// This is called immediately after factory creates the data source
/// Use this to store API clients, credentials, etc. from provider
#[async_trait]
pub trait DataSourceWithConfigure: DataSource {
    async fn configure(
        &mut self,
        ctx: Context,
        request: ConfigureDataSourceRequest,
    ) -> ConfigureDataSourceResponse;
}

pub struct ConfigureDataSourceRequest {
    pub provider_data: Option<Arc<dyn Any + Send + Sync>>,
}

pub struct ConfigureDataSourceResponse {
    pub diagnostics: Vec<Diagnostic>,
}
