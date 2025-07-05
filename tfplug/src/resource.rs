//! Resource trait and related types
//!
//! This module defines the Resource trait and optional traits that resources
//! can implement for additional functionality.

use crate::context::Context;
use crate::schema::Schema;
use crate::types::{
    AttributePath, ClientCapabilities, Deferred, Diagnostic, DynamicValue, RawState,
    ResourceIdentityData,
};
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

/// Base trait for resources - implement CRUD operations
/// Type name should be constant and match the key in Provider.resources()
#[async_trait]
pub trait Resource: Send + Sync {
    /// Type name should be constant (e.g., "proxmox_vm")
    /// MUST match the key used in Provider.resources()
    fn type_name(&self) -> &str;

    /// Called to get resource metadata
    async fn metadata(
        &self,
        ctx: Context,
        request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse;

    /// Called to get resource schema - cache this in your implementation
    async fn schema(&self, ctx: Context, request: ResourceSchemaRequest) -> ResourceSchemaResponse;

    /// Called during plan to validate configuration
    async fn validate(
        &self,
        ctx: Context,
        request: ValidateResourceConfigRequest,
    ) -> ValidateResourceConfigResponse;

    /// Called to create a new resource
    /// MUST populate all attributes in response.new_state (including computed)
    async fn create(&self, ctx: Context, request: CreateResourceRequest) -> CreateResourceResponse;

    /// Called to read current state - used for refresh and after create/update
    /// MUST return accurate current state or None if resource doesn't exist
    async fn read(&self, ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse;

    /// Called to update an existing resource
    /// MUST apply all changes from planned_state to the resource
    async fn update(&self, ctx: Context, request: UpdateResourceRequest) -> UpdateResourceResponse;

    /// Called to delete a resource
    /// MUST remove the resource completely
    async fn delete(&self, ctx: Context, request: DeleteResourceRequest) -> DeleteResourceResponse;
}

// Request/Response types for Resource trait

pub struct ResourceMetadataRequest;

pub struct ResourceMetadataResponse {
    pub type_name: String,
}

pub struct ResourceSchemaRequest;

pub struct ResourceSchemaResponse {
    pub schema: Schema,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ValidateResourceConfigRequest {
    pub type_name: String,
    pub config: DynamicValue,
    pub client_capabilities: ClientCapabilities,
}

pub struct ValidateResourceConfigResponse {
    pub diagnostics: Vec<Diagnostic>,
}

pub struct CreateResourceRequest {
    pub type_name: String,
    pub planned_state: DynamicValue,
    pub config: DynamicValue,
    pub planned_private: Vec<u8>,
    pub provider_meta: Option<DynamicValue>,
}

pub struct CreateResourceResponse {
    pub new_state: DynamicValue,
    pub private: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ReadResourceRequest {
    pub type_name: String,
    pub current_state: DynamicValue,
    pub private: Vec<u8>,
    pub provider_meta: Option<DynamicValue>,
    pub client_capabilities: ClientCapabilities,
    pub current_identity: Option<ResourceIdentityData>,
}

pub struct ReadResourceResponse {
    pub new_state: Option<DynamicValue>,
    pub diagnostics: Vec<Diagnostic>,
    pub private: Vec<u8>,
    pub deferred: Option<Deferred>,
    pub new_identity: Option<ResourceIdentityData>,
}

pub struct UpdateResourceRequest {
    pub type_name: String,
    pub prior_state: DynamicValue,
    pub planned_state: DynamicValue,
    pub config: DynamicValue,
    pub planned_private: Vec<u8>,
    pub provider_meta: Option<DynamicValue>,
    pub planned_identity: Option<ResourceIdentityData>,
}

pub struct UpdateResourceResponse {
    pub new_state: DynamicValue,
    pub private: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,
    pub new_identity: Option<ResourceIdentityData>,
}

pub struct DeleteResourceRequest {
    pub type_name: String,
    pub prior_state: DynamicValue,
    pub planned_private: Vec<u8>,
    pub provider_meta: Option<DynamicValue>,
}

pub struct DeleteResourceResponse {
    pub diagnostics: Vec<Diagnostic>,
}

/// All resources must implement configure to receive provider data
/// This is called immediately after factory creates the resource
/// Use this to store API clients, credentials, etc. from provider
#[async_trait]
pub trait ResourceWithConfigure: Resource {
    async fn configure(
        &mut self,
        ctx: Context,
        request: ConfigureResourceRequest,
    ) -> ConfigureResourceResponse;
}

pub struct ConfigureResourceRequest {
    /// Data from ConfigureProviderResponse.provider_data
    /// Downcast to your provider's specific type
    pub provider_data: Option<Arc<dyn Any + Send + Sync>>,
}

pub struct ConfigureResourceResponse {
    pub diagnostics: Vec<Diagnostic>,
}

/// Optional interface for customizing planning behavior
/// The framework handles most planning logic internally:
/// 1. Marks computed attributes as unknown
/// 2. Applies default values
/// 3. Calls plan modifiers from schema
/// 4. Then calls this if implemented
/// Reference: terraform-plugin-framework/resource/resource.go:124-145
///
/// IMPORTANT: This is called AFTER the framework's automatic planning.
/// Only implement if you need to:
/// - Set unknown values based on other attributes
/// - Add custom requires-replace logic
/// - Modify planned values based on external state
#[async_trait]
pub trait ResourceWithModifyPlan: Resource {
    /// Called during planning to modify proposed changes
    /// Use for computed attributes that affect other attributes
    /// This is called AFTER the framework's default planning logic
    async fn modify_plan(&self, ctx: Context, request: ModifyPlanRequest) -> ModifyPlanResponse;
}

pub struct ModifyPlanRequest {
    pub type_name: String,
    pub config: DynamicValue,
    pub prior_state: DynamicValue,
    pub proposed_new_state: DynamicValue,
    pub prior_private: Vec<u8>,
    pub provider_meta: Option<DynamicValue>,
}

pub struct ModifyPlanResponse {
    pub planned_state: DynamicValue,
    pub requires_replace: Vec<AttributePath>,
    pub planned_private: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Optional interface for handling state upgrades between schema versions
/// If not implemented, the framework behavior is:
/// - If stored version matches current version: return state as-is
/// - If versions differ: return error requiring implementation
/// Reference: terraform-plugin-framework/internal/fwserver/server_upgraderesourcestate.go:58-72
///
/// IMPORTANT: Only implement if you change schema.version
/// The framework automatically handles version checking
#[async_trait]
pub trait ResourceWithUpgradeState: Resource {
    async fn upgrade_state(
        &self,
        ctx: Context,
        request: UpgradeResourceStateRequest,
    ) -> UpgradeResourceStateResponse;
}

pub struct UpgradeResourceStateRequest {
    pub type_name: String,
    pub version: i64,
    pub raw_state: RawState,
}

pub struct UpgradeResourceStateResponse {
    pub upgraded_state: DynamicValue,
    pub diagnostics: Vec<Diagnostic>,
}

/// Optional interface for import functionality
#[async_trait]
pub trait ResourceWithImportState: Resource {
    /// Called during "terraform import" command
    /// Parse the ID and populate full resource state
    async fn import_state(
        &self,
        ctx: Context,
        request: ImportResourceStateRequest,
    ) -> ImportResourceStateResponse;
}

pub struct ImportResourceStateRequest {
    pub type_name: String,
    pub id: String,
    pub client_capabilities: ClientCapabilities,
    pub identity: Option<ResourceIdentityData>,
}

pub struct ImportResourceStateResponse {
    pub imported_resources: Vec<ImportedResource>,
    pub diagnostics: Vec<Diagnostic>,
    pub deferred: Option<Deferred>,
}

pub struct ImportedResource {
    pub type_name: String,
    pub state: DynamicValue,
    pub private: Vec<u8>,
    pub identity: Option<ResourceIdentityData>,
}
