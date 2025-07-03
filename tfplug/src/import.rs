//! Import helpers for simplifying resource import implementations

use crate::context::Context;
use crate::resource::{ImportResourceStateRequest, ImportResourceStateResponse, ImportedResource};
use crate::types::{AttributePath, Dynamic, DynamicValue};
use std::collections::HashMap;

/// Sets the import ID to a specific attribute in state
///
/// This is useful for simple resources where the import ID maps directly to
/// a single attribute in the resource state.
///
/// Example: ID "vm-123" -> state.id = "vm-123"
pub fn import_state_passthrough_id(
    _ctx: &Context,
    attr_path: AttributePath,
    request: &ImportResourceStateRequest,
    response: &mut ImportResourceStateResponse,
) {
    // Create a new state with the ID set to the specified attribute
    let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));

    if let Err(e) = state.set_string(&attr_path, request.id.clone()) {
        response.diagnostics.push(crate::types::Diagnostic {
            severity: crate::types::DiagnosticSeverity::Error,
            summary: format!("Failed to set import ID: {}", e),
            detail: format!(
                "Could not set attribute '{:?}' to value '{}'",
                attr_path, request.id
            ),
            attribute: Some(attr_path),
        });
        return;
    }

    response.imported_resources.push(ImportedResource {
        type_name: request.type_name.clone(),
        state,
        private: Vec::new(),
        identity: request.identity.clone(),
    });
}

/// Handles import with identity support (Terraform 1.12+)
///
/// This function copies an attribute from the identity to the state.
/// It's useful when Terraform provides identity information during import
/// that needs to be mapped to state attributes.
pub fn import_state_passthrough_with_identity(
    _ctx: &Context,
    state_attr_path: AttributePath,
    identity_attr_path: AttributePath,
    request: &ImportResourceStateRequest,
    response: &mut ImportResourceStateResponse,
) {
    let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));

    // Check if identity is provided
    if let Some(ref identity) = request.identity {
        // Try to get the string value from identity
        match identity.identity_data.get_string(&identity_attr_path) {
            Ok(value) => {
                // Set the string value in state
                if let Err(e) = state.set_string(&state_attr_path, value) {
                    response.diagnostics.push(crate::types::Diagnostic {
                        severity: crate::types::DiagnosticSeverity::Error,
                        summary: format!("Failed to copy identity value: {}", e),
                        detail: format!(
                            "Could not copy from identity '{:?}' to state '{:?}'",
                            identity_attr_path, state_attr_path
                        ),
                        attribute: Some(state_attr_path),
                    });
                    return;
                }
            }
            Err(e) => {
                response.diagnostics.push(crate::types::Diagnostic {
                    severity: crate::types::DiagnosticSeverity::Error,
                    summary: format!("Failed to read identity value: {}", e),
                    detail: format!(
                        "Could not read attribute '{:?}' from identity",
                        identity_attr_path
                    ),
                    attribute: Some(identity_attr_path),
                });
                return;
            }
        }
    } else {
        // No identity provided, fall back to ID passthrough
        if let Err(e) = state.set_string(&state_attr_path, request.id.clone()) {
            response.diagnostics.push(crate::types::Diagnostic {
                severity: crate::types::DiagnosticSeverity::Error,
                summary: format!("Failed to set import ID: {}", e),
                detail: format!(
                    "No identity provided, could not set attribute '{:?}' to ID '{}'",
                    state_attr_path, request.id
                ),
                attribute: Some(state_attr_path),
            });
            return;
        }
    }

    response.imported_resources.push(ImportedResource {
        type_name: request.type_name.clone(),
        state,
        private: Vec::new(),
        identity: request.identity.clone(),
    });
}
