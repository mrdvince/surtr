use crate::attribute_type::AttributeType;
use crate::provider::{Attribute, DataSourceSchema, ResourceSchema};
use std::collections::HashMap;

pub struct SchemaBuilder {
    attributes: HashMap<String, Attribute>,
}

pub struct AttributeBuilder {
    attribute: Attribute,
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    pub fn attribute(mut self, name: impl Into<String>, builder: AttributeBuilder) -> Self {
        let attr = builder.build();
        self.attributes.insert(name.into(), attr);
        self
    }

    pub fn build_data_source(self, version: i64) -> DataSourceSchema {
        DataSourceSchema {
            version,
            attributes: self.attributes,
        }
    }

    pub fn build_resource(self, version: i64) -> ResourceSchema {
        ResourceSchema {
            version,
            attributes: self.attributes,
        }
    }
}

impl AttributeBuilder {
    pub fn string(name: impl Into<String>) -> Self {
        Self {
            attribute: Attribute {
                name: name.into(),
                r#type: AttributeType::String,
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: vec![],
                plan_modifiers: vec![],
                default: None,
            },
        }
    }

    pub fn number(name: impl Into<String>) -> Self {
        Self {
            attribute: Attribute {
                name: name.into(),
                r#type: AttributeType::Number,
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: vec![],
                plan_modifiers: vec![],
                default: None,
            },
        }
    }

    pub fn bool(name: impl Into<String>) -> Self {
        Self {
            attribute: Attribute {
                name: name.into(),
                r#type: AttributeType::Bool,
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: vec![],
                plan_modifiers: vec![],
                default: None,
            },
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.attribute.description = desc.into();
        self
    }

    pub fn required(mut self) -> Self {
        self.attribute.required = true;
        self.attribute.optional = false;
        self
    }

    pub fn optional(mut self) -> Self {
        self.attribute.optional = true;
        self.attribute.required = false;
        self
    }

    pub fn computed(mut self) -> Self {
        self.attribute.computed = true;
        self
    }

    pub fn sensitive(mut self) -> Self {
        self.attribute.sensitive = true;
        self
    }

    pub fn validator(mut self, validator: Box<dyn crate::validator::Validator>) -> Self {
        self.attribute.validators.push(validator);
        self
    }

    pub fn plan_modifier(mut self, modifier: Box<dyn crate::plan_modifier::PlanModifier>) -> Self {
        self.attribute.plan_modifiers.push(modifier);
        self
    }

    pub fn default(mut self, default: Box<dyn crate::defaults::Default>) -> Self {
        self.attribute.default = Some(default);
        self
    }

    pub fn build(self) -> Attribute {
        self.attribute
    }
}

impl AttributeBuilder {
    pub fn list(name: impl Into<String>, element_type: AttributeType) -> Self {
        Self {
            attribute: Attribute {
                name: name.into(),
                r#type: AttributeType::List(Box::new(element_type)),
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: vec![],
                plan_modifiers: vec![],
                default: None,
            },
        }
    }

    pub fn set(name: impl Into<String>, element_type: AttributeType) -> Self {
        Self {
            attribute: Attribute {
                name: name.into(),
                r#type: AttributeType::Set(Box::new(element_type)),
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: vec![],
                plan_modifiers: vec![],
                default: None,
            },
        }
    }

    pub fn map(name: impl Into<String>, element_type: AttributeType) -> Self {
        Self {
            attribute: Attribute {
                name: name.into(),
                r#type: AttributeType::Map(Box::new(element_type)),
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: vec![],
                plan_modifiers: vec![],
                default: None,
            },
        }
    }

    pub fn object(name: impl Into<String>, attributes: HashMap<String, AttributeType>) -> Self {
        Self {
            attribute: Attribute {
                name: name.into(),
                r#type: AttributeType::Object(attributes),
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: vec![],
                plan_modifiers: vec![],
                default: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_builder_handles_multiple_attributes() {
        let schema = SchemaBuilder::new()
            .attribute("id", AttributeBuilder::string("id").computed())
            .attribute("name", AttributeBuilder::string("name").required())
            .attribute("enabled", AttributeBuilder::bool("enabled").optional())
            .attribute("count", AttributeBuilder::number("count").optional())
            .build_data_source(1);

        assert_eq!(schema.version, 1);
        assert_eq!(schema.attributes.len(), 4);

        let id_attr = &schema.attributes["id"];
        assert!(id_attr.computed);
        assert!(!id_attr.required);

        let name_attr = &schema.attributes["name"];
        assert!(name_attr.required);
        assert!(!name_attr.optional);

        let enabled_attr = &schema.attributes["enabled"];
        assert!(enabled_attr.optional);
        assert!(!enabled_attr.required);
    }

    #[test]
    fn attribute_builder_mutually_exclusive_required_optional() {
        let required_attr = AttributeBuilder::string("test").required().build();
        assert!(required_attr.required);
        assert!(!required_attr.optional);

        let optional_attr = AttributeBuilder::string("test").optional().build();
        assert!(optional_attr.optional);
        assert!(!optional_attr.required);

        let req_then_opt = AttributeBuilder::string("test")
            .required()
            .optional()
            .build();
        assert!(req_then_opt.optional);
        assert!(!req_then_opt.required);

        let opt_then_req = AttributeBuilder::string("test")
            .optional()
            .required()
            .build();
        assert!(opt_then_req.required);
        assert!(!opt_then_req.optional);
    }

    #[test]
    fn sensitive_attribute_configuration() {
        let schema = SchemaBuilder::new()
            .attribute(
                "password",
                AttributeBuilder::string("password")
                    .required()
                    .sensitive()
                    .description("API password"),
            )
            .build_resource(0);

        let password_attr = &schema.attributes["password"];
        assert!(password_attr.sensitive);
        assert!(password_attr.required);
        assert_eq!(password_attr.description, "API password");
    }

    #[test]
    fn computed_attributes_common_pattern() {
        let schema = SchemaBuilder::new()
            .attribute("id", AttributeBuilder::string("id").computed())
            .attribute(
                "created_at",
                AttributeBuilder::string("created_at").computed(),
            )
            .attribute(
                "updated_at",
                AttributeBuilder::string("updated_at").computed(),
            )
            .build_resource(1);

        for attr_name in ["id", "created_at", "updated_at"] {
            let attr = &schema.attributes[attr_name];
            assert!(attr.computed);
            assert!(!attr.required);
            assert!(!attr.optional);
        }
    }

    #[test]
    fn schema_version_propagation() {
        let data_source = SchemaBuilder::new()
            .attribute("test", AttributeBuilder::string("test"))
            .build_data_source(42);
        assert_eq!(data_source.version, 42);

        let resource = SchemaBuilder::new()
            .attribute("test", AttributeBuilder::string("test"))
            .build_resource(99);
        assert_eq!(resource.version, 99);
    }
}
