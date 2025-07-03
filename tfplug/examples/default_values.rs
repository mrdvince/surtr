use tfplug::{
    defaults::StaticDefault,
    plan_modifier::RequiresReplace,
    schema::{Default, DefaultRequest, DefaultResponse},
    types::{Dynamic, DynamicValue},
    validator::StringLengthValidator,
    AttributeBuilder, AttributeType, SchemaBuilder,
};

fn main() {
    // Example 1: Static string default
    let _schema = SchemaBuilder::new()
        .attribute(
            AttributeBuilder::new("description", AttributeType::String)
                .optional()
                .computed()
                .description("Description of the resource")
                .default(StaticDefault::string("No description provided"))
                .build(),
        )
        .build();

    println!("Created schema with default description");

    // Example 2: Static boolean default
    let _schema = SchemaBuilder::new()
        .attribute(
            AttributeBuilder::new("enabled", AttributeType::Bool)
                .optional()
                .computed()
                .description("Whether the resource is enabled")
                .default(StaticDefault::bool(true))
                .build(),
        )
        .build();

    println!("Created schema with default enabled=true");

    // Example 3: Static number default
    let _schema = SchemaBuilder::new()
        .attribute(
            AttributeBuilder::new("timeout", AttributeType::Number)
                .optional()
                .computed()
                .description("Timeout in seconds")
                .default(StaticDefault::number(30.0))
                .build(),
        )
        .build();

    println!("Created schema with default timeout=30");

    // Example 4: Custom default implementation
    struct CurrentTimestampDefault;

    impl Default for CurrentTimestampDefault {
        fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
            use std::time::{SystemTime, UNIX_EPOCH};

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            DefaultResponse {
                value: DynamicValue::new(Dynamic::String(timestamp.to_string())),
            }
        }

        fn description(&self) -> String {
            "defaults to current Unix timestamp".to_string()
        }
    }

    let _schema = SchemaBuilder::new()
        .attribute(
            AttributeBuilder::new("created_at", AttributeType::String)
                .optional()
                .computed()
                .description("Creation timestamp")
                .default(Box::new(CurrentTimestampDefault))
                .build(),
        )
        .build();

    println!("Created schema with dynamic timestamp default");

    // Example 5: Environment-based default
    struct EnvironmentDefault {
        env_var: String,
        fallback: String,
    }

    impl Default for EnvironmentDefault {
        fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
            let value = std::env::var(&self.env_var).unwrap_or_else(|_| self.fallback.clone());

            DefaultResponse {
                value: DynamicValue::new(Dynamic::String(value)),
            }
        }

        fn description(&self) -> String {
            format!("defaults to ${} or \"{}\"", self.env_var, self.fallback)
        }
    }

    let _schema = SchemaBuilder::new()
        .attribute(
            AttributeBuilder::new("region", AttributeType::String)
                .optional()
                .computed()
                .description("AWS region")
                .default(Box::new(EnvironmentDefault {
                    env_var: "AWS_DEFAULT_REGION".to_string(),
                    fallback: "us-east-1".to_string(),
                }))
                .build(),
        )
        .build();

    println!("Created schema with environment-based default");

    // Example 6: Combining defaults with validators and plan modifiers

    let _schema = SchemaBuilder::new()
        .attribute(
            AttributeBuilder::new("instance_type", AttributeType::String)
                .optional()
                .computed()
                .description("EC2 instance type")
                .default(StaticDefault::string("t2.micro"))
                .validator(StringLengthValidator::min(5))
                .plan_modifier(Box::new(RequiresReplace))
                .build(),
        )
        .build();

    println!("Created schema combining defaults, validators, and plan modifiers");
}
