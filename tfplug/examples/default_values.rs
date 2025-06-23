use tfplug::{
    AttributeBuilder, DefaultRequest, DefaultResponse, Dynamic, SchemaBuilder, StaticBool,
    StaticNumber, StaticString,
};

fn main() {
    // Example 1: Static string default
    let _schema = SchemaBuilder::new()
        .attribute(
            "description",
            AttributeBuilder::string("description")
                .optional()
                .computed()
                .description("Description of the resource")
                .default(Box::new(StaticString::new("No description provided"))),
        )
        .build_resource(1);

    println!("Created schema with default description");

    // Example 2: Static boolean default
    let _schema = SchemaBuilder::new()
        .attribute(
            "enabled",
            AttributeBuilder::bool("enabled")
                .optional()
                .computed()
                .description("Whether the resource is enabled")
                .default(Box::new(StaticBool::new(true))),
        )
        .build_resource(1);

    println!("Created schema with default enabled=true");

    // Example 3: Static number default
    let _schema = SchemaBuilder::new()
        .attribute(
            "timeout",
            AttributeBuilder::number("timeout")
                .optional()
                .computed()
                .description("Timeout in seconds")
                .default(Box::new(StaticNumber::new(30.0))),
        )
        .build_resource(1);

    println!("Created schema with default timeout=30");

    // Example 4: Custom default implementation
    struct CurrentTimestampDefault;

    impl tfplug::Default for CurrentTimestampDefault {
        fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
            use std::time::{SystemTime, UNIX_EPOCH};

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            DefaultResponse {
                value: Dynamic::String(timestamp.to_string()),
            }
        }

        fn description(&self) -> String {
            "defaults to current Unix timestamp".to_string()
        }
    }

    let _schema = SchemaBuilder::new()
        .attribute(
            "created_at",
            AttributeBuilder::string("created_at")
                .optional()
                .computed()
                .description("Creation timestamp")
                .default(Box::new(CurrentTimestampDefault)),
        )
        .build_resource(1);

    println!("Created schema with dynamic timestamp default");

    // Example 5: Environment-based default
    struct EnvironmentDefault {
        env_var: String,
        fallback: String,
    }

    impl tfplug::Default for EnvironmentDefault {
        fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
            let value = std::env::var(&self.env_var).unwrap_or_else(|_| self.fallback.clone());

            DefaultResponse {
                value: Dynamic::String(value),
            }
        }

        fn description(&self) -> String {
            format!("defaults to ${} or \"{}\"", self.env_var, self.fallback)
        }
    }

    let _schema = SchemaBuilder::new()
        .attribute(
            "region",
            AttributeBuilder::string("region")
                .optional()
                .computed()
                .description("AWS region")
                .default(Box::new(EnvironmentDefault {
                    env_var: "AWS_DEFAULT_REGION".to_string(),
                    fallback: "us-east-1".to_string(),
                })),
        )
        .build_resource(1);

    println!("Created schema with environment-based default");

    // Example 6: Combining defaults with validators and plan modifiers
    use tfplug::{validator::StringLengthValidator, RequiresReplaceIfChanged};

    let _schema = SchemaBuilder::new()
        .attribute(
            "instance_type",
            AttributeBuilder::string("instance_type")
                .optional()
                .computed()
                .description("EC2 instance type")
                .default(Box::new(StaticString::new("t2.micro")))
                .validator(Box::new(StringLengthValidator {
                    min: Some(5),
                    max: None,
                }))
                .plan_modifier(Box::new(RequiresReplaceIfChanged)),
        )
        .build_resource(1);

    println!("Created schema combining defaults, validators, and plan modifiers");
}
