//! Example demonstrating comprehensive plan modifier usage with the new tfplug API
//!
//! This example shows:
//! - Built-in plan modifiers (RequiresReplace, UseStateForUnknown, etc.)
//! - Custom plan modifier implementation
//! - Conditional plan modifiers
//! - Complex attribute paths
//! - Combining multiple plan modifiers

use tfplug::defaults::StaticDefault;
use tfplug::plan_modifier::{
    NormalizeCase, PreventUpdate, RequiresReplace, RequiresReplaceIf, SetDefault,
    UseStateForUnknown,
};
use tfplug::schema::{
    AttributeBuilder, AttributeType, PlanModifier, PlanModifierRequest, PlanModifierResponse,
    Schema, SchemaBuilder,
};
use tfplug::types::{AttributePath, Diagnostic, Dynamic, DynamicValue};

/// Custom plan modifier that enforces specific naming conventions
struct EnforceNamingConvention {
    prefix: String,
}

impl EnforceNamingConvention {
    fn create(prefix: &str) -> Box<dyn PlanModifier> {
        Box::new(Self {
            prefix: prefix.to_string(),
        })
    }
}

impl PlanModifier for EnforceNamingConvention {
    fn description(&self) -> String {
        format!("enforces names to start with '{}'", self.prefix)
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // Check if the value is a string and doesn't start with the prefix
        if let Dynamic::String(name) = &request.plan_value.value {
            if !name.starts_with(&self.prefix) {
                // Automatically prepend the prefix
                let corrected_name = format!("{}-{}", self.prefix, name);
                response.plan_value = DynamicValue::new(Dynamic::String(corrected_name));

                // Add a warning diagnostic
                response.diagnostics.push(
                    Diagnostic::warning(
                        "Name adjusted to meet convention",
                        format!(
                            "The name '{}' was automatically prefixed to '{}-{}'",
                            name, self.prefix, name
                        ),
                    )
                    .with_attribute(request.path),
                );
            }
        }

        response
    }
}

/// Custom plan modifier that validates and normalizes port numbers
struct NormalizePort;

impl NormalizePort {
    fn create() -> Box<dyn PlanModifier> {
        Box::new(Self)
    }
}

impl PlanModifier for NormalizePort {
    fn description(&self) -> String {
        "normalizes port numbers to valid range (1-65535)".to_string()
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let mut response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        if let Dynamic::Number(port) = &request.plan_value.value {
            let port_int = *port as i32;

            if port_int < 1 || port_int > 65535 {
                // Clamp to valid range
                let normalized = port_int.clamp(1, 65535);
                response.plan_value = DynamicValue::new(Dynamic::Number(normalized as f64));

                response.diagnostics.push(
                    Diagnostic::warning(
                        "Port number normalized",
                        format!("Port {} was adjusted to {}", port_int, normalized),
                    )
                    .with_attribute(request.path),
                );
            }
        }

        response
    }
}

/// Custom plan modifier that tracks changes for auditing
struct AuditChanges;

impl AuditChanges {
    fn create() -> Box<dyn PlanModifier> {
        Box::new(Self)
    }
}

impl PlanModifier for AuditChanges {
    fn description(&self) -> String {
        "tracks attribute changes for auditing".to_string()
    }

    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse {
        let response = PlanModifierResponse {
            plan_value: request.plan_value.clone(),
            requires_replace: false,
            diagnostics: Vec::new(),
        };

        // Only audit if there's a state value and it's changing
        if !request.state_value.is_null() && request.state_value.value != request.plan_value.value {
            println!(
                "[AUDIT] Attribute '{}' changing from {:?} to {:?}",
                request
                    .path
                    .steps
                    .iter()
                    .map(|s| match s {
                        tfplug::types::AttributePathStep::AttributeName(name) => name.clone(),
                        tfplug::types::AttributePathStep::ElementKeyString(key) =>
                            format!("[{}]", key),
                        tfplug::types::AttributePathStep::ElementKeyInt(idx) =>
                            format!("[{}]", idx),
                    })
                    .collect::<Vec<_>>()
                    .join("."),
                request.state_value.value,
                request.plan_value.value
            );
        }

        response
    }
}

/// Example demonstrating how to build a schema with various plan modifiers
fn build_server_resource_schema() -> Schema {
    SchemaBuilder::new()
        .version(1)
        // Immutable identifier that requires replacement on change
        .attribute(
            AttributeBuilder::new("instance_id", AttributeType::String)
                .required()
                .plan_modifier(RequiresReplace::create())
                .plan_modifier(AuditChanges::create())
                .description("Instance ID - changing this forces resource replacement")
                .build(),
        )
        // Name with enforced convention and normalization
        .attribute(
            AttributeBuilder::new("name", AttributeType::String)
                .required()
                .plan_modifier(EnforceNamingConvention::create("srv"))
                .plan_modifier(NormalizeCase::lower())
                .plan_modifier(AuditChanges::create())
                .description("Server name - automatically prefixed with 'srv-' and lowercased")
                .build(),
        )
        // Environment with conditional replacement
        .attribute(
            AttributeBuilder::new("environment", AttributeType::String)
                .optional()
                .default(StaticDefault::string("development"))
                .plan_modifier(RequiresReplaceIf::create(
                    "changing from production",
                    |req| {
                        // Require replacement when moving FROM production
                        if let Dynamic::String(old) = &req.state_value.value {
                            old == "production"
                        } else {
                            false
                        }
                    },
                ))
                .plan_modifier(AuditChanges::create())
                .description("Environment - changing from production requires replacement")
                .build(),
        )
        // Computed field that preserves state when unknown
        .attribute(
            AttributeBuilder::new("private_ip", AttributeType::String)
                .computed()
                .plan_modifier(UseStateForUnknown::create())
                .description("Private IP - preserves previous value when unknown")
                .build(),
        )
        // Immutable field that cannot be changed after creation
        .attribute(
            AttributeBuilder::new("availability_zone", AttributeType::String)
                .required()
                .plan_modifier(PreventUpdate::create())
                .plan_modifier(AuditChanges::create())
                .description("Availability zone - cannot be changed after creation")
                .build(),
        )
        // Port with normalization and default
        .attribute(
            AttributeBuilder::new("port", AttributeType::Number)
                .optional()
                .plan_modifier(SetDefault::number(8080.0))
                .plan_modifier(NormalizePort::create())
                .plan_modifier(AuditChanges::create())
                .description("Server port - defaults to 8080, normalized to valid range")
                .build(),
        )
        // Tags with case normalization
        .attribute(
            AttributeBuilder::new("tags", AttributeType::Map(Box::new(AttributeType::String)))
                .optional()
                .plan_modifier(AuditChanges::create())
                .description("Resource tags - key-value pairs")
                .build(),
        )
        // Security groups list
        .attribute(
            AttributeBuilder::new(
                "security_groups",
                AttributeType::List(Box::new(AttributeType::String)),
            )
            .optional()
            .plan_modifier(AuditChanges::create())
            .description("Security group IDs")
            .build(),
        )
        // Subnet ID that requires replacement on change
        .attribute(
            AttributeBuilder::new("subnet_id", AttributeType::String)
                .required()
                .plan_modifier(RequiresReplace::create())
                .plan_modifier(AuditChanges::create())
                .description("Subnet ID - changing requires resource replacement")
                .build(),
        )
        .build()
}

/// Example showing how plan modifiers work during the planning phase
fn demonstrate_plan_modifier_behavior() {
    println!("\nPlan Modifier Behavior Examples:");
    println!("================================\n");

    // Example 1: RequiresReplace in action
    {
        let modifier = RequiresReplace::create();
        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("i-new-instance".to_string())),
            state_value: DynamicValue::new(Dynamic::String("i-old-instance".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("i-new-instance".to_string())),
            path: AttributePath::new("instance_id"),
        };
        let response = modifier.modify(request);
        println!("1. RequiresReplace:");
        println!("   - Old value: i-old-instance");
        println!("   - New value: i-new-instance");
        println!("   - Requires replacement: {}\n", response.requires_replace);
    }

    // Example 2: UseStateForUnknown preserving computed values
    {
        let modifier = UseStateForUnknown::create();
        let request = PlanModifierRequest {
            config_value: DynamicValue::unknown(),
            state_value: DynamicValue::new(Dynamic::String("10.0.1.42".to_string())),
            plan_value: DynamicValue::unknown(),
            path: AttributePath::new("private_ip"),
        };
        let response = modifier.modify(request);
        println!("2. UseStateForUnknown:");
        println!("   - State value: 10.0.1.42");
        println!("   - Plan value: (unknown)");
        println!(
            "   - Result: {}\n",
            if let Dynamic::String(s) = &response.plan_value.value {
                s
            } else {
                "(unknown)"
            }
        );
    }

    // Example 3: SetDefault providing values
    {
        let modifier = SetDefault::string("development");
        let request = PlanModifierRequest {
            config_value: DynamicValue::null(),
            state_value: DynamicValue::null(),
            plan_value: DynamicValue::null(),
            path: AttributePath::new("environment"),
        };
        let response = modifier.modify(request);
        println!("3. SetDefault:");
        println!("   - Config value: (null)");
        println!(
            "   - Default applied: {}\n",
            if let Dynamic::String(s) = &response.plan_value.value {
                s
            } else {
                "(none)"
            }
        );
    }

    // Example 4: NormalizeCase
    {
        let modifier = NormalizeCase::lower();
        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("SRV-Production".to_string())),
            state_value: DynamicValue::null(),
            plan_value: DynamicValue::new(Dynamic::String("SRV-Production".to_string())),
            path: AttributePath::new("name"),
        };
        let response = modifier.modify(request);
        println!("4. NormalizeCase:");
        println!("   - Input: SRV-Production");
        println!(
            "   - Normalized: {}\n",
            if let Dynamic::String(s) = &response.plan_value.value {
                s
            } else {
                "(error)"
            }
        );
    }

    // Example 5: Custom EnforceNamingConvention
    {
        let modifier = EnforceNamingConvention::create("srv");
        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("webserver".to_string())),
            state_value: DynamicValue::null(),
            plan_value: DynamicValue::new(Dynamic::String("webserver".to_string())),
            path: AttributePath::new("name"),
        };
        let response = modifier.modify(request);
        println!("5. EnforceNamingConvention:");
        println!("   - Input: webserver");
        println!(
            "   - Prefixed: {}",
            if let Dynamic::String(s) = &response.plan_value.value {
                s
            } else {
                "(error)"
            }
        );
        if !response.diagnostics.is_empty() {
            println!("   - Warning: {}", response.diagnostics[0].summary);
        }
        println!();
    }

    // Example 6: Complex path handling
    {
        let modifier = PreventUpdate::create();
        let mut path = AttributePath::new("security_groups");
        path = path.index(0);

        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("sg-new".to_string())),
            state_value: DynamicValue::new(Dynamic::String("sg-old".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("sg-new".to_string())),
            path,
        };
        let response = modifier.modify(request);
        println!("6. PreventUpdate with nested path:");
        println!("   - Path: security_groups[0]");
        println!("   - Attempting to change from 'sg-old' to 'sg-new'");
        if !response.diagnostics.is_empty() {
            println!("   - Error: {}", response.diagnostics[0].summary);
            println!("   - Detail: {}", response.diagnostics[0].detail);
        }
        println!();
    }

    // Example 7: Conditional replacement
    {
        let modifier = RequiresReplaceIf::create("changing from production", |req| {
            if let Dynamic::String(old) = &req.state_value.value {
                old == "production"
            } else {
                false
            }
        });

        // Moving from production to staging - requires replacement
        let request = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("staging".to_string())),
            state_value: DynamicValue::new(Dynamic::String("production".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("staging".to_string())),
            path: AttributePath::new("environment"),
        };
        let response = modifier.modify(request);
        println!("7. RequiresReplaceIf (from production):");
        println!("   - Old: production, New: staging");
        println!("   - Requires replacement: {}", response.requires_replace);

        // Moving from development to staging - no replacement needed
        let request2 = PlanModifierRequest {
            config_value: DynamicValue::new(Dynamic::String("staging".to_string())),
            state_value: DynamicValue::new(Dynamic::String("development".to_string())),
            plan_value: DynamicValue::new(Dynamic::String("staging".to_string())),
            path: AttributePath::new("environment"),
        };
        let response2 = modifier.modify(request2);
        println!("   - Old: development, New: staging");
        println!(
            "   - Requires replacement: {}\n",
            response2.requires_replace
        );
    }
}

// Example usage demonstration
fn main() {
    println!("Plan Modifier Usage Example");
    println!("==========================\n");

    // Demonstrate plan modifier behavior
    println!("Built-in Plan Modifiers:");
    println!("- RequiresReplace: Forces resource replacement on change");
    println!("- RequiresReplaceIf: Conditional replacement based on custom logic");
    println!("- UseStateForUnknown: Preserves state value for unknown planned values");
    println!("- PreventUpdate: Blocks updates to immutable attributes");
    println!("- SetDefault: Provides default values for optional attributes");
    println!("- NormalizeCase: Normalizes string values to upper/lowercase\n");

    println!("Custom Plan Modifiers:");
    println!("- EnforceNamingConvention: Ensures names follow specific patterns");
    println!("- NormalizePort: Validates and clamps port numbers to valid range");
    println!("- AuditChanges: Tracks and logs attribute changes\n");

    println!("Key Features Demonstrated:");
    println!("1. Multiple plan modifiers can be chained on a single attribute");
    println!("2. Plan modifiers receive config, state, and planned values");
    println!("3. Modifiers can alter planned values and add diagnostics");
    println!("4. Conditional logic can determine when replacement is needed");
    println!("5. Complex paths are supported for nested attributes\n");

    // Show the schema structure
    println!("Example Resource Schema:");
    let schema = build_server_resource_schema();
    println!("Version: {}", schema.version);
    println!("Attributes: {}", schema.block.attributes.len());
    println!(
        "Total elements in schema: {} attributes",
        schema.block.attributes.len()
    );

    // Demonstrate actual plan modifier behavior
    demonstrate_plan_modifier_behavior();
}
