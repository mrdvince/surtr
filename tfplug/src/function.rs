//! Function trait for provider functions

use crate::context::Context;
use crate::types::{Diagnostic, DynamicValue, FunctionError};
use async_trait::async_trait;

/// Function trait for provider functions
#[async_trait]
pub trait Function: Send + Sync {
    /// Get function metadata
    async fn metadata(
        &self,
        ctx: Context,
        request: FunctionMetadataRequest,
    ) -> FunctionMetadataResponse;

    /// Get function definition (parameters, return type)
    async fn definition(
        &self,
        ctx: Context,
        request: FunctionDefinitionRequest,
    ) -> FunctionDefinitionResponse;

    /// Execute the function
    async fn call(&self, ctx: Context, request: CallFunctionRequest) -> CallFunctionResponse;
}

// Request/Response Types
pub struct FunctionMetadataRequest;

pub struct FunctionMetadataResponse {
    pub name: String,
}

pub struct FunctionDefinitionRequest;

pub struct FunctionDefinitionResponse {
    pub definition: FunctionDefinition,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct FunctionDefinition {
    pub parameters: Vec<Parameter>,
    pub variadic_parameter: Option<Parameter>,
    pub return_type: ReturnType,
    pub summary: String,
    pub description: String,
    pub deprecation_message: Option<String>,
}

pub struct Parameter {
    pub name: String,
    pub type_: Vec<u8>, // Encoded type
    pub allow_null_value: bool,
    pub allow_unknown_values: bool,
    pub description: String,
}

pub struct ReturnType {
    pub type_: Vec<u8>, // Encoded type
}

pub struct CallFunctionRequest {
    pub name: String,
    pub arguments: Vec<DynamicValue>,
}

pub struct CallFunctionResponse {
    pub result: Option<DynamicValue>,
    pub error: Option<FunctionError>,
}
