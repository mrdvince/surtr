//! Ephemeral resource trait for temporary resources

use crate::context::Context;
use crate::schema::Schema;
use crate::types::{ClientCapabilities, Deferred, Diagnostic, DynamicValue};
use async_trait::async_trait;

/// EphemeralResource trait for resources with temporary lifecycle
#[async_trait]
pub trait EphemeralResource: Send + Sync {
    /// Get ephemeral resource metadata
    async fn metadata(
        &self,
        ctx: Context,
        request: EphemeralResourceMetadataRequest,
    ) -> EphemeralResourceMetadataResponse;

    /// Get ephemeral resource schema
    async fn schema(
        &self,
        ctx: Context,
        request: EphemeralResourceSchemaRequest,
    ) -> EphemeralResourceSchemaResponse;

    /// Validate configuration
    async fn validate(
        &self,
        ctx: Context,
        request: ValidateEphemeralResourceConfigRequest,
    ) -> ValidateEphemeralResourceConfigResponse;

    /// Open/create the ephemeral resource (e.g., create connection)
    async fn open(
        &self,
        ctx: Context,
        request: OpenEphemeralResourceRequest,
    ) -> OpenEphemeralResourceResponse;

    /// Renew the ephemeral resource (e.g., refresh token)
    async fn renew(
        &self,
        ctx: Context,
        request: RenewEphemeralResourceRequest,
    ) -> RenewEphemeralResourceResponse;

    /// Close the ephemeral resource (e.g., close connection)
    async fn close(
        &self,
        ctx: Context,
        request: CloseEphemeralResourceRequest,
    ) -> CloseEphemeralResourceResponse;
}

// Request/Response Types
pub struct EphemeralResourceMetadataRequest;

pub struct EphemeralResourceMetadataResponse {
    pub type_name: String,
}

pub struct EphemeralResourceSchemaRequest;

pub struct EphemeralResourceSchemaResponse {
    pub schema: Schema,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ValidateEphemeralResourceConfigRequest {
    pub type_name: String,
    pub config: DynamicValue,
}

pub struct ValidateEphemeralResourceConfigResponse {
    pub diagnostics: Vec<Diagnostic>,
}

pub struct OpenEphemeralResourceRequest {
    pub type_name: String,
    pub config: DynamicValue,
    pub client_capabilities: ClientCapabilities,
}

pub struct OpenEphemeralResourceResponse {
    pub diagnostics: Vec<Diagnostic>,
    pub renew_at: Option<std::time::SystemTime>,
    pub result: DynamicValue,
    pub private: Option<Vec<u8>>,
    pub deferred: Option<Deferred>,
}

pub struct RenewEphemeralResourceRequest {
    pub type_name: String,
    pub private: Option<Vec<u8>>,
}

pub struct RenewEphemeralResourceResponse {
    pub diagnostics: Vec<Diagnostic>,
    pub renew_at: Option<std::time::SystemTime>,
    pub private: Option<Vec<u8>>,
}

pub struct CloseEphemeralResourceRequest {
    pub type_name: String,
    pub private: Option<Vec<u8>>,
}

pub struct CloseEphemeralResourceResponse {
    pub diagnostics: Vec<Diagnostic>,
}
