//! Protocol buffer types for Terraform Plugin Protocol v6.9
//!
//! This module includes and re-exports the generated protobuf types from the
//! tfplugin6.9.proto file. The protobuf code is generated at build time by
//! tonic_build and included here.
//!
//! # Usage
//!
//! Access protobuf types through this module to interact with the Terraform Plugin Protocol:
//!
//! ```rust,ignore
//! use tfplug::proto;
//!
//! // Request/Response types are nested in their respective modules
//! let request = proto::get_provider_schema::Request::default();
//! let response = proto::get_provider_schema::Response::default();
//!
//! // Some types have the same names as tfplug framework types
//! // Always use proto:: prefix to disambiguate
//! let proto_dynamic = proto::DynamicValue::default();  // Protobuf type
//! let framework_dynamic = tfplug::DynamicValue::new(); // Framework type
//! ```
//!
//! # Type Naming
//!
//! The protobuf generation follows these patterns:
//! - Top-level messages become structs (e.g., `DynamicValue`, `Schema`)
//! - RPC methods have nested `Request` and `Response` types in snake_case modules
//!   (e.g., `get_provider_schema::Request`, `read_resource::Response`)
//! - Nested messages are in sub-modules (e.g., `diagnostic::Severity`)
//! - The gRPC service trait is available as `provider_server::Provider`
//!
//! # Common Types
//!
//! - `DynamicValue` - Encoded Terraform values (msgpack or JSON)
//! - `Diagnostic` - Error and warning messages
//! - `AttributePath` - Path to nested attributes
//! - `Schema` - Resource/data source schema definition
//! - `RawState` - Raw state for migrations
//!
//! Note: Some protobuf types have the same names as tfplug framework types.
//! To avoid conflicts, always use the `proto::` prefix when referring to protobuf types.

// Include the generated protobuf code from the build output directory
// The file name is based on the proto package name (tfplugin6)
include!(concat!(env!("OUT_DIR"), "/tfplugin6.rs"));

// Re-export the gRPC service trait and server
pub use provider_server::{Provider as ProviderService, ProviderServer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proto_types_accessible() {
        // Verify that key types are accessible with proto:: prefix
        let _ = DynamicValue::default();
        let _ = Diagnostic::default();
        let _ = AttributePath::default();
        let _ = ServerCapabilities::default();
        let _ = ClientCapabilities::default();
    }

    #[test]
    fn test_nested_types_accessible() {
        // Verify nested types are accessible
        let _ = diagnostic::Severity::Invalid;
        let _ = attribute_path::step::Selector::AttributeName("test".to_string());
        let _ = schema::nested_block::NestingMode::Single;
    }

    #[test]
    fn test_request_response_types() {
        // Verify request/response types are accessible
        let _ = get_provider_schema::Request::default();
        let _ = get_provider_schema::Response::default();
        let _ = read_resource::Request::default();
        let _ = read_resource::Response::default();
    }
}
