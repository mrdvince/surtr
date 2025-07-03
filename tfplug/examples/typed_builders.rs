use tfplug::{
    attribute_type::AttributeType, provider::ResourceSchema, Config, Dynamic,
    RequiresReplaceIfChanged, StateBuilder,
};

fn main() {
    example_fluent_resource_schema();
    example_config_extraction();
    example_state_building();
}

fn example_fluent_resource_schema() {
    println!("=== Fluent Resource Schema Builder ===");

    let schema = ResourceSchema::builder()
        .version(0)
        .required_string("realm", "Realm/domain name")
        .required_string("type", "Authentication type (e.g., 'openid')")
        .required_string("issuer_url", "OpenID issuer URL")
        .required_string("client_id", "OAuth/OpenID client ID")
        .required_sensitive_string("client_key", "OAuth/OpenID client secret")
        .optional_string("username_claim", "OpenID claim used as username")
        .optional_bool_with_default("autocreate", "Automatically create users", false)
        .optional_bool("default", "Set as default authentication realm")
        .optional_string("comment", "Description/comment for the realm")
        .optional_list(
            "allowed_groups",
            "List of allowed groups",
            AttributeType::String,
        )
        .optional_map(
            "custom_claims",
            "Custom claim mappings",
            AttributeType::String,
        )
        .id_attribute()
        .with_timestamps()
        .custom_attribute("region", |attr| {
            attr.optional()
                .description("Region for the resource")
                .plan_modifier(Box::new(RequiresReplaceIfChanged))
        })
        .build();

    println!("Created schema with {} attributes", schema.attributes.len());
}

fn example_config_extraction() {
    println!("\n=== Config Extraction Helpers ===");

    let mut config = Config::new();
    config
        .values
        .insert("realm".to_string(), Dynamic::String("my-realm".to_string()));
    config
        .values
        .insert("enabled".to_string(), Dynamic::Bool(true));
    config
        .values
        .insert("port".to_string(), Dynamic::Number(8080.0));
    config.values.insert(
        "tags".to_string(),
        Dynamic::List(vec![
            Dynamic::String("prod".to_string()),
            Dynamic::String("api".to_string()),
        ]),
    );

    let realm = config.require_string("realm").unwrap();
    println!("Required realm: {}", realm);

    let description = config.get_string("description");
    println!("Optional description: {:?}", description);

    let enabled = config.get_bool("enabled");
    println!("Enabled: {:?}", enabled);

    let port = config.require_number("port").unwrap();
    println!("Port: {}", port);

    let tags = config.get_list("tags");
    println!("Tags: {:?}", tags);

    match config.require_string("missing_field") {
        Ok(_) => println!("Should not happen"),
        Err(e) => println!("Expected error: {}", e),
    }
}

fn example_state_building() {
    println!("\n=== State Building Helpers ===");

    let config = create_sample_config();

    let state = StateBuilder::from_config(&config)
        .string("id", "realm-123")
        .bool("active", true)
        .number("created_timestamp", 1640995200.0)
        .build();

    println!("State values: {:?}", state.values);

    let state_from_scratch = StateBuilder::new()
        .string("id", "resource-456")
        .string("name", "My Resource")
        .bool("enabled", true)
        .number("timeout", 30.0)
        .list(
            "features",
            vec![
                Dynamic::String("feature1".to_string()),
                Dynamic::String("feature2".to_string()),
            ],
        )
        .build();

    println!("State from scratch: {:?}", state_from_scratch.values);
}

fn create_sample_config() -> Config {
    let mut config = Config::new();
    config.values.insert(
        "realm".to_string(),
        Dynamic::String("test-realm".to_string()),
    );
    config
        .values
        .insert("type".to_string(), Dynamic::String("openid".to_string()));
    config
}
