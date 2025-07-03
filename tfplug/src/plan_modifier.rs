//! Plan modifiers for attribute modification during planning
//!
//! This module provides built-in plan modifiers that control how Terraform plans changes
//! to resource attributes. Plan modifiers are executed during the planning phase and can:
//!
//! - Mark attributes as requiring resource replacement
//! - Preserve state values for unknown computed attributes
//! - Prevent updates to immutable attributes
//! - Set default values for optional attributes
//! - Normalize attribute values (e.g., case normalization)
//!
//! # Examples
//!
//! ```no_run
//! use tfplug::schema::{AttributeBuilder, AttributeType};
//! use tfplug::plan_modifier::{RequiresReplace, UseStateForUnknown};
//!
//! // Attribute that requires replacement when changed
//! let instance_type = AttributeBuilder::new("instance_type", AttributeType::String)
//!     .required()
//!     .plan_modifier(RequiresReplace::create())
//!     .build();
//!
//! // Computed attribute that preserves state when unknown
//! let private_ip = AttributeBuilder::new("private_ip", AttributeType::String)
//!     .computed()
//!     .plan_modifier(UseStateForUnknown::create())
//!     .build();
//! ```

use crate::schema::{PlanModifier, PlanModifierRequest, PlanModifierResponse};
use crate::types::{Diagnostic, Dynamic, DynamicValue};

/// RequiresReplace marks an attribute as requiring resource replacement when changed
pub struct RequiresReplace;

impl RequiresReplace {
    /// Create a new RequiresReplace modifier
    pub fn create() -> Box<dyn PlanModifier> {
        Box::new(Self)
    }
}

impl PlanModifier for RequiresReplace {
    fn description(&self) -> String {
        "requires replacement when value changes".to_string()
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // If state exists and values differ, require replacement
        if !request.state_value.is_null() && request.state_value.value != request.plan_value.value {
            response.requires_replace = true;
        }

        response
    }
}

/// RequiresReplaceIf conditionally requires replacement based on a condition
pub struct RequiresReplaceIf<F>
where
    F: Fn(&PlanModifierRequest) -> bool + Send + Sync,
{
    condition: F,
    description: String,
}

impl<F> RequiresReplaceIf<F>
where
    F: Fn(&PlanModifierRequest) -> bool + Send + Sync + 'static,
{
    /// Create a new RequiresReplaceIf modifier with a condition function
    pub fn create(description: &str, condition: F) -> Box<dyn PlanModifier> {
        Box::new(Self {
            condition,
            description: description.to_string(),
        })
    }
}

impl<F> PlanModifier for RequiresReplaceIf<F>
where
    F: Fn(&PlanModifierRequest) -> bool + Send + Sync,
{
    fn description(&self) -> String {
        format!("requires replacement if {}", self.description)
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // If state exists and condition is met, require replacement
        if !request.state_value.is_null() && (self.condition)(&request) {
            response.requires_replace = true;
        }

        response
    }
}

/// UseStateForUnknown uses the previous state value when the planned value is unknown
pub struct UseStateForUnknown;

impl UseStateForUnknown {
    /// Create a new UseStateForUnknown modifier
    pub fn create() -> Box<dyn PlanModifier> {
        Box::new(Self)
    }
}

impl PlanModifier for UseStateForUnknown {
    fn description(&self) -> String {
        "uses previous state value when planned value is unknown".to_string()
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // If planned value is unknown and state has a known value, use the state value
        if request.plan_value.is_unknown()
            && !request.state_value.is_null()
            && !request.state_value.is_unknown()
        {
            response.plan_value = request.state_value.clone();
        }

        response
    }
}

/// PreventUpdate prevents an attribute from being updated after creation
pub struct PreventUpdate;

impl PreventUpdate {
    /// Create a new PreventUpdate modifier
    pub fn create() -> Box<dyn PlanModifier> {
        Box::new(Self)
    }
}

impl PlanModifier for PreventUpdate {
    fn description(&self) -> String {
        "prevents updates after resource creation".to_string()
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // If state exists and values differ, generate an error
        if !request.state_value.is_null() && request.state_value.value != request.plan_value.value {
            response.diagnostics.push(
                Diagnostic::error(
                    "Attribute cannot be updated",
                    format!(
                        "The attribute at path '{}' cannot be changed after resource creation",
                        request
                            .path
                            .steps
                            .iter()
                            .map(|s| match s {
                                crate::types::AttributePathStep::AttributeName(name) =>
                                    name.clone(),
                                crate::types::AttributePathStep::ElementKeyString(key) =>
                                    format!("[{}]", key),
                                crate::types::AttributePathStep::ElementKeyInt(idx) =>
                                    format!("[{}]", idx),
                            })
                            .collect::<Vec<_>>()
                            .join(".")
                    ),
                )
                .with_attribute(request.path),
            );
        }

        response
    }
}

/// SetDefault sets a default value when the config value is null
pub struct SetDefault {
    default_value: Dynamic,
}

impl SetDefault {
    /// Create a new SetDefault modifier with a static value
    pub fn create(value: Dynamic) -> Box<dyn PlanModifier> {
        Box::new(Self {
            default_value: value,
        })
    }

    /// Create a modifier that sets a string default
    pub fn string(value: &str) -> Box<dyn PlanModifier> {
        Box::new(Self {
            default_value: Dynamic::String(value.to_string()),
        })
    }

    /// Create a modifier that sets a number default
    pub fn number(value: f64) -> Box<dyn PlanModifier> {
        Box::new(Self {
            default_value: Dynamic::Number(value),
        })
    }

    /// Create a modifier that sets a boolean default
    pub fn bool(value: bool) -> Box<dyn PlanModifier> {
        Box::new(Self {
            default_value: Dynamic::Bool(value),
        })
    }
}

impl PlanModifier for SetDefault {
    fn description(&self) -> String {
        format!("sets default value to {:?}", self.default_value)
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // If config value is null, use the default
        if request.config_value.is_null() {
            response.plan_value = DynamicValue::new(self.default_value.clone());
        }

        response
    }
}

/// NormalizeCase normalizes string values to a specific case
pub enum CaseNormalization {
    Lower,
    Upper,
}

pub struct NormalizeCase {
    case: CaseNormalization,
}

impl NormalizeCase {
    /// Create a modifier that normalizes to lowercase
    pub fn lower() -> Box<dyn PlanModifier> {
        Box::new(Self {
            case: CaseNormalization::Lower,
        })
    }

    /// Create a modifier that normalizes to uppercase
    pub fn upper() -> Box<dyn PlanModifier> {
        Box::new(Self {
            case: CaseNormalization::Upper,
        })
    }
}

impl PlanModifier for NormalizeCase {
    fn description(&self) -> String {
        match self.case {
            CaseNormalization::Lower => "normalizes string to lowercase".to_string(),
            CaseNormalization::Upper => "normalizes string to uppercase".to_string(),
        }
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // Normalize string values
        if let Dynamic::String(s) = &request.plan_value.value {
            let normalized = match self.case {
                CaseNormalization::Lower => s.to_lowercase(),
                CaseNormalization::Upper => s.to_uppercase(),
            };
            response.plan_value = DynamicValue::new(Dynamic::String(normalized));
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AttributePath;

    #[test]
    fn requires_replace_detects_change() {
        let modifier = RequiresReplace::create();

        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("new".to_string())),
            state_value: DynamicValue::new(Dynamic::String("old".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("new".to_string())),
            path: AttributePath::new("test"),
        };

        let response = modifier.modify(request);
        assert!(response.requires_replace);
    }

    #[test]
    fn requires_replace_no_change() {
        let modifier = RequiresReplace::create();

        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("same".to_string())),
            state_value: DynamicValue::new(Dynamic::String("same".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("same".to_string())),
            path: AttributePath::new("test"),
        };

        let response = modifier.modify(request);
        assert!(!response.requires_replace);
    }

    #[test]
    fn requires_replace_if_condition_met() {
        let modifier = RequiresReplaceIf::create("value starts with 'prod'", |req| {
            if let Dynamic::String(s) = &req.plan_value.value {
                s.starts_with("prod")
            } else {
                false
            }
        });

        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("prod-server".to_string())),
            state_value: DynamicValue::new(Dynamic::String("dev-server".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("prod-server".to_string())),
            path: AttributePath::new("environment"),
        };

        let response = modifier.modify(request);
        assert!(response.requires_replace);
    }

    #[test]
    fn use_state_for_unknown() {
        let modifier = UseStateForUnknown::create();

        let request = PlanModifierRequest {
            config_value: DynamicValue::unknown(),
            state_value: DynamicValue::new(Dynamic::String("existing".to_string())),
            plan_value: DynamicValue::unknown(),
            path: AttributePath::new("computed_field"),
        };

        let response = modifier.modify(request);
        assert_eq!(
            response.plan_value.value,
            Dynamic::String("existing".to_string())
        );
    }

    #[test]
    fn prevent_update_blocks_changes() {
        let modifier = PreventUpdate::create();

        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("new".to_string())),
            state_value: DynamicValue::new(Dynamic::String("old".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("new".to_string())),
            path: AttributePath::new("immutable_field"),
        };

        let response = modifier.modify(request);
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0]
            .summary
            .contains("cannot be updated"));
    }

    #[test]
    fn set_default_when_null() {
        let modifier = SetDefault::string("default-value");

        let request = PlanModifierRequest {
            config_value: DynamicValue::null(),
            state_value: DynamicValue::null(),
            plan_value: DynamicValue::null(),
            path: AttributePath::new("optional_field"),
        };

        let response = modifier.modify(request);
        assert_eq!(
            response.plan_value.value,
            Dynamic::String("default-value".to_string())
        );
    }

    #[test]
    fn normalize_case_to_lower() {
        let modifier = NormalizeCase::lower();

        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("MixedCase".to_string())),
            state_value: DynamicValue::null(),
            plan_value: DynamicValue::new(Dynamic::String("MixedCase".to_string())),
            path: AttributePath::new("normalized_field"),
        };

        let response = modifier.modify(request);
        assert_eq!(
            response.plan_value.value,
            Dynamic::String("mixedcase".to_string())
        );
    }

    #[test]
    fn complex_path_handling() {
        let modifier = PreventUpdate::create();

        let mut path = AttributePath::new("config");
        path = path.attribute("server").index(0).attribute("port");

        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::Number(8080.0)),
            state_value: DynamicValue::new(Dynamic::Number(80.0)),
            plan_value: DynamicValue::new(Dynamic::Number(8080.0)),
            path,
        };

        let response = modifier.modify(request);
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0]
            .detail
            .contains("config.server.[0].port"));
    }
}
