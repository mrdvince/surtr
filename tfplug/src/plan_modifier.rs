use crate::types::{Diagnostics, Dynamic};

#[derive(Debug, Clone)]
pub struct PlanModifyRequest {
    pub state: Dynamic,
    pub plan: Dynamic,
    pub config: Dynamic,
    pub attribute_path: String,
}

#[derive(Debug, Clone)]
pub struct PlanModifyResponse {
    pub plan_value: Dynamic,
    pub requires_replace: bool,
    pub diagnostics: Diagnostics,
}

/// Trait for modifying terraform plan behavior
///
/// Plan modifiers run after Terraform has generated a plan and can:
/// - Modify the planned value
/// - Mark an attribute as requiring replacement
/// - Add warnings or errors to the plan
pub trait PlanModifier: Send + Sync {
    /// Modify the plan for an attribute
    fn modify_plan(&self, request: PlanModifyRequest) -> PlanModifyResponse;
}

/// Marks an attribute as requiring replacement when it changes
pub struct RequiresReplaceIfChanged;

impl PlanModifier for RequiresReplaceIfChanged {
    fn modify_plan(&self, request: PlanModifyRequest) -> PlanModifyResponse {
        let requires_replace = !matches!(
            (&request.state, &request.plan),
            (Dynamic::Null, Dynamic::Null) | (Dynamic::Unknown, _) | (_, Dynamic::Unknown)
        ) && !values_equal(&request.state, &request.plan);

        PlanModifyResponse {
            plan_value: request.plan,
            requires_replace,
            diagnostics: Diagnostics::new(),
        }
    }
}

/// A plan modifier that uses the current state value when the planned value is unknown
///
/// This is particularly useful for computed attributes that should retain their value
/// during planning when Terraform doesn't know what the new value will be.
pub struct UseStateForUnknown;

impl PlanModifier for UseStateForUnknown {
    fn modify_plan(&self, request: PlanModifyRequest) -> PlanModifyResponse {
        let plan_value = match &request.plan {
            // Handle both Unknown and Null for computed values
            // (Unknown may be decoded as Null due to msgpack limitations)
            Dynamic::Unknown | Dynamic::Null => {
                // Only use state if it's not null
                match &request.state {
                    Dynamic::Null => request.plan,
                    _ => request.state.clone(),
                }
            }
            _ => request.plan,
        };

        PlanModifyResponse {
            plan_value,
            requires_replace: false,
            diagnostics: Diagnostics::new(),
        }
    }
}

pub struct RequiresReplaceIf<F>
where
    F: Fn(&PlanModifyRequest) -> bool + Send + Sync,
{
    predicate: F,
    description: String,
}

impl<F> RequiresReplaceIf<F>
where
    F: Fn(&PlanModifyRequest) -> bool + Send + Sync,
{
    pub fn new(predicate: F, description: impl Into<String>) -> Self {
        Self {
            predicate,
            description: description.into(),
        }
    }
}

impl<F> PlanModifier for RequiresReplaceIf<F>
where
    F: Fn(&PlanModifyRequest) -> bool + Send + Sync,
{
    fn modify_plan(&self, request: PlanModifyRequest) -> PlanModifyResponse {
        let mut diagnostics = Diagnostics::new();
        let requires_replace = (self.predicate)(&request);

        if requires_replace {
            diagnostics.add_warning(
                format!(
                    "Attribute '{}' requires resource replacement",
                    request.attribute_path
                ),
                Some(&self.description),
            );
        }

        PlanModifyResponse {
            plan_value: request.plan,
            requires_replace,
            diagnostics,
        }
    }
}

/// Helper function to compare two Dynamic values for equality
fn values_equal(a: &Dynamic, b: &Dynamic) -> bool {
    match (a, b) {
        (Dynamic::Null, Dynamic::Null) => true,
        (Dynamic::Bool(a), Dynamic::Bool(b)) => a == b,
        (Dynamic::Number(a), Dynamic::Number(b)) => (a - b).abs() < f64::EPSILON,
        (Dynamic::String(a), Dynamic::String(b)) => a == b,
        (Dynamic::List(a), Dynamic::List(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        (Dynamic::Map(a), Dynamic::Map(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|v2| values_equal(v, v2)))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn requires_replace_if_changed_does_not_trigger_on_same_value() {
        let modifier = RequiresReplaceIfChanged;

        let request = PlanModifyRequest {
            state: Dynamic::String("hello".to_string()),
            plan: Dynamic::String("hello".to_string()),
            config: Dynamic::String("hello".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);

        assert!(!response.requires_replace);
        assert_eq!(response.diagnostics.errors.len(), 0);
    }

    #[test]
    fn requires_replace_if_changed_triggers_on_different_value() {
        let modifier = RequiresReplaceIfChanged;

        let request = PlanModifyRequest {
            state: Dynamic::String("hello".to_string()),
            plan: Dynamic::String("world".to_string()),
            config: Dynamic::String("world".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);

        assert!(response.requires_replace);
        assert_eq!(response.diagnostics.errors.len(), 0);
    }

    #[test]
    fn requires_replace_if_changed_ignores_null_to_null() {
        let modifier = RequiresReplaceIfChanged;

        let request = PlanModifyRequest {
            state: Dynamic::Null,
            plan: Dynamic::Null,
            config: Dynamic::Null,
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);

        assert!(!response.requires_replace);
    }

    #[test]
    fn requires_replace_if_changed_ignores_unknown_values() {
        let modifier = RequiresReplaceIfChanged;

        // Unknown in state
        let request = PlanModifyRequest {
            state: Dynamic::Unknown,
            plan: Dynamic::String("value".to_string()),
            config: Dynamic::String("value".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);
        assert!(!response.requires_replace);

        // Unknown in plan
        let request = PlanModifyRequest {
            state: Dynamic::String("value".to_string()),
            plan: Dynamic::Unknown,
            config: Dynamic::String("value".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);
        assert!(!response.requires_replace);
    }

    #[test]
    fn values_equal_handles_all_types() {
        // Numbers
        assert!(values_equal(&Dynamic::Number(42.0), &Dynamic::Number(42.0)));
        assert!(!values_equal(
            &Dynamic::Number(42.0),
            &Dynamic::Number(43.0)
        ));

        // Booleans
        assert!(values_equal(&Dynamic::Bool(true), &Dynamic::Bool(true)));
        assert!(!values_equal(&Dynamic::Bool(true), &Dynamic::Bool(false)));

        // Lists
        let list1 = Dynamic::List(vec![Dynamic::String("a".to_string()), Dynamic::Number(1.0)]);
        let list2 = Dynamic::List(vec![Dynamic::String("a".to_string()), Dynamic::Number(1.0)]);
        let list3 = Dynamic::List(vec![Dynamic::String("b".to_string()), Dynamic::Number(1.0)]);
        assert!(values_equal(&list1, &list2));
        assert!(!values_equal(&list1, &list3));

        // Maps
        let mut map1 = HashMap::new();
        map1.insert("key".to_string(), Dynamic::String("value".to_string()));
        let mut map2 = HashMap::new();
        map2.insert("key".to_string(), Dynamic::String("value".to_string()));
        let mut map3 = HashMap::new();
        map3.insert("key".to_string(), Dynamic::String("different".to_string()));

        assert!(values_equal(
            &Dynamic::Map(map1.clone()),
            &Dynamic::Map(map2)
        ));
        assert!(!values_equal(&Dynamic::Map(map1), &Dynamic::Map(map3)));
    }

    #[test]
    fn custom_plan_modifier_can_modify_planned_value() {
        struct DefaultIfNullModifier {
            default_value: String,
        }

        impl PlanModifier for DefaultIfNullModifier {
            fn modify_plan(&self, request: PlanModifyRequest) -> PlanModifyResponse {
                let mut diagnostics = Diagnostics::new();

                let plan_value = match &request.plan {
                    Dynamic::Null => {
                        diagnostics.add_warning(
                            format!("Setting default value for {}", request.attribute_path),
                            Some(format!("Using default: '{}'", self.default_value)),
                        );
                        Dynamic::String(self.default_value.clone())
                    }
                    _ => request.plan,
                };

                PlanModifyResponse {
                    plan_value,
                    requires_replace: false,
                    diagnostics,
                }
            }
        }

        let modifier = DefaultIfNullModifier {
            default_value: "default-value".to_string(),
        };

        let request = PlanModifyRequest {
            state: Dynamic::Null,
            plan: Dynamic::Null,
            config: Dynamic::Null,
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);

        match response.plan_value {
            Dynamic::String(s) => assert_eq!(s, "default-value"),
            _ => panic!("Expected string value"),
        }

        assert_eq!(response.diagnostics.warnings.len(), 1);
        assert!(!response.requires_replace);
    }

    #[test]
    fn use_state_for_unknown_preserves_state_when_unknown() {
        let modifier = UseStateForUnknown;

        let request = PlanModifyRequest {
            state: Dynamic::String("existing-value".to_string()),
            plan: Dynamic::Unknown,
            config: Dynamic::String("configured-value".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);

        match response.plan_value {
            Dynamic::String(s) => assert_eq!(s, "existing-value"),
            _ => panic!("Expected string value from state"),
        }

        assert!(!response.requires_replace);
    }

    #[test]
    fn use_state_for_unknown_uses_plan_when_known() {
        let modifier = UseStateForUnknown;

        let request = PlanModifyRequest {
            state: Dynamic::String("existing-value".to_string()),
            plan: Dynamic::String("new-value".to_string()),
            config: Dynamic::String("new-value".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);

        match response.plan_value {
            Dynamic::String(s) => assert_eq!(s, "new-value"),
            _ => panic!("Expected plan value"),
        }

        assert!(!response.requires_replace);
    }

    #[test]
    fn requires_replace_if_triggers_on_condition() {
        let modifier = RequiresReplaceIf::new(
            |req| {
                // Require replacement if changing from non-empty to empty string
                matches!((&req.state, &req.plan),
                    (Dynamic::String(old), Dynamic::String(new)) if !old.is_empty() && new.is_empty()
                )
            },
            "Cannot change to empty string without replacement",
        );

        // Test case that should trigger replacement
        let request = PlanModifyRequest {
            state: Dynamic::String("has-value".to_string()),
            plan: Dynamic::String("".to_string()),
            config: Dynamic::String("".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);
        assert!(response.requires_replace);
        assert_eq!(response.diagnostics.warnings.len(), 1);

        // Test case that should NOT trigger replacement
        let request = PlanModifyRequest {
            state: Dynamic::String("".to_string()),
            plan: Dynamic::String("new-value".to_string()),
            config: Dynamic::String("new-value".to_string()),
            attribute_path: "test.field".to_string(),
        };

        let response = modifier.modify_plan(request);
        assert!(!response.requires_replace);
        assert_eq!(response.diagnostics.warnings.len(), 0);
    }
}
