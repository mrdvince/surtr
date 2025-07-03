//! Example of using plan modifiers in tfplug
//!
//! This example demonstrates how to use all built-in plan modifiers
//! when building resource schemas. Plan modifiers control how Terraform
//! plans changes to resource attributes.
//!
//! Available plan modifiers:
//! - RequiresReplace: Forces resource replacement when attribute changes
//! - RequiresReplaceIf: Conditionally forces replacement based on custom logic
//! - UseStateForUnknown: Preserves state value for unknown computed attributes
//! - PreventUpdate: Prevents attribute updates after resource creation
//! - SetDefault: Sets default values for optional attributes
//! - NormalizeCase: Normalizes string case (upper/lower)

use tfplug::plan_modifier::{
    NormalizeCase, PreventUpdate, RequiresReplace, RequiresReplaceIf, SetDefault,
    UseStateForUnknown,
};
use tfplug::schema::{AttributeBuilder, AttributeType, SchemaBuilder};
use tfplug::types::Dynamic;

fn main() {
    // Example 1: Attribute that always requires replacement when changed
    // Use case: Instance type changes require creating a new instance
    let instance_type = AttributeBuilder::new("instance_type", AttributeType::String)
        .description("Instance type (changing requires replacement)")
        .required()
        .plan_modifier(RequiresReplace::create())
        .build();

    // Example 2: Attribute that conditionally requires replacement
    // Use case: Changing to production environment requires new resource
    let environment = AttributeBuilder::new("environment", AttributeType::String)
        .description("Environment (prod changes require replacement)")
        .required()
        .plan_modifier(RequiresReplaceIf::create(
            "changing to production",
            |request| {
                // Require replacement when changing TO production
                if let Dynamic::String(new_env) = &request.plan_value.value {
                    if let Dynamic::String(old_env) = &request.state_value.value {
                        new_env == "production" && old_env != "production"
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
        ))
        .build();

    // Example 3: Computed attribute that preserves state when unknown
    // Use case: Keep existing IP address when Terraform can't determine new value
    let private_ip = AttributeBuilder::new("private_ip", AttributeType::String)
        .description("Private IP address assigned to the instance")
        .computed()
        .plan_modifier(UseStateForUnknown::create())
        .build();

    // Example 4: Attribute that cannot be updated after creation
    // Use case: Encryption keys should never change
    let encryption_key = AttributeBuilder::new("encryption_key", AttributeType::String)
        .description("Encryption key (immutable after creation)")
        .optional()
        .sensitive()
        .plan_modifier(PreventUpdate::create())
        .build();

    // Example 5: Optional attributes with default values
    let region = AttributeBuilder::new("region", AttributeType::String)
        .description("AWS region")
        .optional()
        .plan_modifier(SetDefault::string("us-east-1"))
        .build();

    let port = AttributeBuilder::new("port", AttributeType::Number)
        .description("Server port")
        .optional()
        .plan_modifier(SetDefault::number(8080.0))
        .build();

    let enable_monitoring = AttributeBuilder::new("enable_monitoring", AttributeType::Bool)
        .description("Enable monitoring")
        .optional()
        .plan_modifier(SetDefault::bool(true))
        .build();

    // Example 6: String normalization modifiers
    let username = AttributeBuilder::new("username", AttributeType::String)
        .description("Username (normalized to lowercase)")
        .required()
        .plan_modifier(NormalizeCase::lower())
        .build();

    let project_code = AttributeBuilder::new("project_code", AttributeType::String)
        .description("Project code (normalized to uppercase)")
        .required()
        .plan_modifier(NormalizeCase::upper())
        .build();

    // Example 7: Combining multiple plan modifiers
    // The modifiers are applied in the order they're added
    let tier = AttributeBuilder::new("tier", AttributeType::String)
        .description("Service tier (defaults to 'BASIC', normalized to uppercase)")
        .optional()
        .plan_modifier(SetDefault::string("basic")) // First: set default if null
        .plan_modifier(NormalizeCase::upper()) // Then: normalize to uppercase
        .build();

    // Example 8: Complex conditional replacement
    // Use case: Disk size can increase but not decrease
    let disk_size = AttributeBuilder::new("disk_size", AttributeType::Number)
        .description("Disk size in GB (cannot be decreased)")
        .required()
        .plan_modifier(RequiresReplaceIf::create(
            "disk size decreases",
            |request| {
                if let (Dynamic::Number(new_size), Dynamic::Number(old_size)) =
                    (&request.plan_value.value, &request.state_value.value)
                {
                    new_size < old_size // Require replacement if new size is smaller
                } else {
                    false
                }
            },
        ))
        .build();

    // Build a complete schema with all examples
    let schema = SchemaBuilder::new()
        .version(1)
        .description("Comprehensive example of plan modifiers")
        .attribute(instance_type)
        .attribute(environment)
        .attribute(private_ip)
        .attribute(encryption_key)
        .attribute(region)
        .attribute(port)
        .attribute(enable_monitoring)
        .attribute(username)
        .attribute(project_code)
        .attribute(tier)
        .attribute(disk_size)
        .build();

    // Display information about the schema
    println!(
        "Created schema with {} attributes",
        schema.block.attributes.len()
    );
    println!("\nPlan modifier summary:");

    for attr in &schema.block.attributes {
        println!("\n  {}:", attr.name);
        println!("    Type: {:?}", attr.r#type);
        println!(
            "    Required: {}, Optional: {}, Computed: {}",
            attr.required, attr.optional, attr.computed
        );
        println!("    Plan modifiers: {}", attr.plan_modifiers.len());

        for modifier in &attr.plan_modifiers {
            println!("      - {}", modifier.description());
        }
    }

    println!("\nKey concepts demonstrated:");
    println!("  - RequiresReplace: Forces resource recreation on change");
    println!("  - RequiresReplaceIf: Conditional resource recreation");
    println!("  - UseStateForUnknown: Preserves values for computed attributes");
    println!("  - PreventUpdate: Makes attributes immutable after creation");
    println!("  - SetDefault: Provides default values for optional attributes");
    println!("  - NormalizeCase: Ensures consistent string casing");
    println!("  - Modifier chaining: Apply multiple modifiers in sequence");
}
