//! tfplug - Terraform Plugin Framework for Rust
//!
//! A framework for building Terraform providers in Rust, implementing the
//! Terraform Plugin Protocol v6.9.

// Core modules
pub mod context;
pub mod error;
pub mod schema;
pub mod types;

// Provider API modules
pub mod data_source;
pub mod ephemeral;
pub mod function;
pub mod provider;
pub mod resource;

// Helper modules
pub mod defaults;
pub mod import;
pub mod plan_modifier;
pub mod validator;

// Framework implementation modules - to be implemented
pub mod grpc;
pub mod proto;
pub mod server;

// Re-exports for convenience
pub use context::Context;
pub use data_source::{DataSource, DataSourceWithConfigure};
pub use error::{Result, TfplugError};
pub use import::{import_state_passthrough_id, import_state_passthrough_with_identity};
pub use provider::{Provider, ProviderMetadataRequest, ProviderMetadataResponse};
pub use resource::{Resource, ResourceWithConfigure, ResourceWithModifyPlan};
pub use schema::{AttributeBuilder, AttributeType, Schema, SchemaBuilder};
pub use server::{serve, serve_default, LogLevel, ServerConfig};
pub use types::{Dynamic, DynamicValue, PrivateStateData};

// Convenience macro for main function
#[macro_export]
macro_rules! serve_provider {
    ($provider:expr) => {
        #[tokio::main]
        async fn main() -> $crate::Result<()> {
            $crate::serve($provider, $crate::ServerConfig::default()).await
        }
    };
    ($provider:expr, $config:expr) => {
        #[tokio::main]
        async fn main() -> $crate::Result<()> {
            $crate::serve($provider, $config).await
        }
    };
}
