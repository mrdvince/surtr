//! Example demonstrating tfplug's type-safe builder patterns
//!
//! This example shows:
//! - Building schemas with AttributeBuilder and SchemaBuilder
//! - Using defaults and plan modifiers  
//! - Working with DynamicValue for configuration and state
//! - Type-safe access to values using AttributePath

use std::collections::HashMap;
use tfplug::{
    defaults::StaticDefault, plan_modifier::RequiresReplace, AttributeBuilder, AttributeType,
    Dynamic, DynamicValue, SchemaBuilder,
};

fn main() {
    example_fluent_resource_schema();
    example_config_extraction();
    example_state_building();
}

fn example_fluent_resource_schema() {
    println!("=== Fluent Resource Schema Builder ===");

    let schema = SchemaBuilder::new()
        .attribute(
            AttributeBuilder::new("realm", AttributeType::String)
                .required()
                .description("Realm/domain name")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("type", AttributeType::String)
                .required()
                .description("Authentication type (e.g., 'openid')")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("issuer_url", AttributeType::String)
                .required()
                .description("OpenID issuer URL")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("client_id", AttributeType::String)
                .required()
                .description("OAuth/OpenID client ID")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("client_key", AttributeType::String)
                .required()
                .sensitive()
                .description("OAuth/OpenID client secret")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("username_claim", AttributeType::String)
                .optional()
                .description("OpenID claim used as username")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("autocreate", AttributeType::Bool)
                .optional()
                .description("Automatically create users")
                .default(StaticDefault::create(Dynamic::Bool(false)))
                .build(),
        )
        .attribute(
            AttributeBuilder::new("default", AttributeType::Bool)
                .optional()
                .description("Set as default authentication realm")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("comment", AttributeType::String)
                .optional()
                .description("Description/comment for the realm")
                .build(),
        )
        .attribute(
            AttributeBuilder::new(
                "allowed_groups",
                AttributeType::List(Box::new(AttributeType::String)),
            )
            .optional()
            .description("List of allowed groups")
            .build(),
        )
        .attribute(
            AttributeBuilder::new(
                "custom_claims",
                AttributeType::Map(Box::new(AttributeType::String)),
            )
            .optional()
            .description("Custom claim mappings")
            .build(),
        )
        .attribute(
            AttributeBuilder::new("id", AttributeType::String)
                .computed()
                .description("Resource ID")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("created_at", AttributeType::String)
                .computed()
                .description("Creation timestamp")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("updated_at", AttributeType::String)
                .computed()
                .description("Last update timestamp")
                .build(),
        )
        .attribute(
            AttributeBuilder::new("region", AttributeType::String)
                .optional()
                .description("Region for the resource")
                .plan_modifier(RequiresReplace::create())
                .build(),
        )
        .build();

    println!(
        "Created schema with {} attributes",
        schema.block.attributes.len()
    );
}

fn example_config_extraction() {
    println!("\n=== Config Extraction Helpers ===");

    // Create a config with some values
    let mut map = HashMap::new();
    map.insert("realm".to_string(), Dynamic::String("my-realm".to_string()));
    map.insert("enabled".to_string(), Dynamic::Bool(true));
    map.insert("port".to_string(), Dynamic::Number(8080.0));
    map.insert(
        "tags".to_string(),
        Dynamic::List(vec![
            Dynamic::String("prod".to_string()),
            Dynamic::String("api".to_string()),
        ]),
    );

    let config = DynamicValue::new(Dynamic::Map(map));

    // Access values using AttributePath
    let realm = config
        .get_string(&tfplug::types::AttributePath::new("realm"))
        .unwrap();
    println!("Required realm: {}", realm);

    // Check for optional field
    let description = config
        .get_string(&tfplug::types::AttributePath::new("description"))
        .ok();
    println!("Optional description: {:?}", description);

    let enabled = config
        .get_bool(&tfplug::types::AttributePath::new("enabled"))
        .unwrap();
    println!("Enabled: {}", enabled);

    let port = config
        .get_number(&tfplug::types::AttributePath::new("port"))
        .unwrap();
    println!("Port: {}", port);

    let tags = config
        .get_list(&tfplug::types::AttributePath::new("tags"))
        .unwrap();
    println!("Tags: {:?}", tags);

    match config.get_string(&tfplug::types::AttributePath::new("missing_field")) {
        Ok(_) => println!("Should not happen"),
        Err(e) => println!("Expected error: {}", e),
    }
}

fn example_state_building() {
    println!("\n=== State Building Helpers ===");

    let config = create_sample_config();

    // Build state by copying config and adding computed values
    let mut state_map = HashMap::new();

    // Copy values from config if it's a map
    if let Dynamic::Map(config_map) = &config.value {
        for (k, v) in config_map {
            state_map.insert(k.clone(), v.clone());
        }
    }

    // Add computed values
    state_map.insert("id".to_string(), Dynamic::String("realm-123".to_string()));
    state_map.insert("active".to_string(), Dynamic::Bool(true));
    state_map.insert(
        "created_timestamp".to_string(),
        Dynamic::Number(1640995200.0),
    );

    let state = DynamicValue::new(Dynamic::Map(state_map));
    println!("State values: {:?}", state.value);

    // Build state from scratch
    let mut state_map = HashMap::new();
    state_map.insert(
        "id".to_string(),
        Dynamic::String("resource-456".to_string()),
    );
    state_map.insert(
        "name".to_string(),
        Dynamic::String("My Resource".to_string()),
    );
    state_map.insert("enabled".to_string(), Dynamic::Bool(true));
    state_map.insert("timeout".to_string(), Dynamic::Number(30.0));
    state_map.insert(
        "features".to_string(),
        Dynamic::List(vec![
            Dynamic::String("feature1".to_string()),
            Dynamic::String("feature2".to_string()),
        ]),
    );

    let state_from_scratch = DynamicValue::new(Dynamic::Map(state_map));
    println!("State from scratch: {:?}", state_from_scratch.value);
}

fn create_sample_config() -> DynamicValue {
    let mut map = HashMap::new();
    map.insert(
        "realm".to_string(),
        Dynamic::String("test-realm".to_string()),
    );
    map.insert("type".to_string(), Dynamic::String("openid".to_string()));
    DynamicValue::new(Dynamic::Map(map))
}
