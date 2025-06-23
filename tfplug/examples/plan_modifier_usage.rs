use tfplug::{
    plan_modifier::{RequiresReplaceIf, RequiresReplaceIfChanged, UseStateForUnknown},
    AttributeBuilder, Diagnostics, Dynamic, PlanModifier, PlanModifyRequest, PlanModifyResponse,
    SchemaBuilder,
};

fn main() {
    // Example 1: Using RequiresReplaceIfChanged for immutable attributes
    SchemaBuilder::new()
        .attribute(
            "instance_type",
            AttributeBuilder::string("instance_type")
                .required()
                .description("The type of instance (e.g., 't2.micro')")
                .plan_modifier(Box::new(RequiresReplaceIfChanged)),
        )
        .build_resource(1);

    println!("Created schema with immutable instance_type attribute");

    // Example 2: Using UseStateForUnknown for computed attributes
    SchemaBuilder::new()
        .attribute(
            "created_at",
            AttributeBuilder::string("created_at")
                .computed()
                .description("Timestamp when the resource was created")
                .plan_modifier(Box::new(UseStateForUnknown)),
        )
        .build_resource(1);

    println!("Created schema with computed created_at attribute that preserves state");

    // Example 3: Using RequiresReplaceIf with custom logic
    SchemaBuilder::new()
        .attribute(
            "encryption_key",
            AttributeBuilder::string("encryption_key")
                .optional()
                .sensitive()
                .description("Encryption key for the resource")
                .plan_modifier(Box::new(RequiresReplaceIf::new(
                    |req| {
                        // Require replacement if changing from encrypted to unencrypted
                        matches!(
                            (&req.state, &req.plan),
                            (Dynamic::String(old), Dynamic::Null) if !old.is_empty()
                        )
                    },
                    "Cannot remove encryption once enabled",
                ))),
        )
        .build_resource(1);

    println!("Created schema with conditional replacement for encryption_key");

    // Example 4: Custom plan modifier
    struct NormalizePathModifier;

    impl PlanModifier for NormalizePathModifier {
        fn modify_plan(&self, request: PlanModifyRequest) -> PlanModifyResponse {
            let plan_value = match &request.plan {
                Dynamic::String(path) => {
                    // Normalize the path by removing trailing slashes
                    let normalized = path.trim_end_matches('/');
                    if normalized != path {
                        Dynamic::String(normalized.to_string())
                    } else {
                        request.plan
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

    SchemaBuilder::new()
        .attribute(
            "base_path",
            AttributeBuilder::string("base_path")
                .optional()
                .description("Base path for the resource")
                .plan_modifier(Box::new(NormalizePathModifier)),
        )
        .build_resource(1);

    println!("Created schema with custom path normalization modifier");
}
