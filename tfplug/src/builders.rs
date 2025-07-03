use crate::{
    attribute_type::AttributeType,
    plan_modifier::UseStateForUnknown,
    provider::{DataSourceSchema, ResourceSchema},
    AttributeBuilder, SchemaBuilder, StaticBool, StaticNumber, StaticString,
};
use std::collections::HashMap;

pub struct FluentResourceSchemaBuilder {
    builder: SchemaBuilder,
    version: i64,
}

#[allow(clippy::derivable_impls)]
impl Default for FluentResourceSchemaBuilder {
    fn default() -> Self {
        Self {
            builder: SchemaBuilder::new(),
            version: 0,
        }
    }
}

impl FluentResourceSchemaBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(mut self, version: i64) -> Self {
        self.version = version;
        self
    }

    pub fn required_string(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::string(name)
                .required()
                .description(description),
        );
        self
    }

    pub fn required_sensitive_string(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::string(name)
                .required()
                .sensitive()
                .description(description),
        );
        self
    }

    pub fn optional_string(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::string(name)
                .optional()
                .description(description),
        );
        self
    }

    pub fn optional_string_with_default(
        mut self,
        name: &str,
        description: &str,
        default: &str,
    ) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::string(name)
                .optional()
                .computed()
                .description(description)
                .default(Box::new(StaticString::new(default))),
        );
        self
    }

    pub fn required_bool(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::bool(name)
                .required()
                .description(description),
        );
        self
    }

    pub fn optional_bool(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::bool(name)
                .optional()
                .description(description),
        );
        self
    }

    pub fn optional_bool_with_default(
        mut self,
        name: &str,
        description: &str,
        default: bool,
    ) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::bool(name)
                .optional()
                .computed()
                .description(description)
                .default(Box::new(StaticBool::new(default))),
        );
        self
    }

    pub fn required_number(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::number(name)
                .required()
                .description(description),
        );
        self
    }

    pub fn optional_number(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::number(name)
                .optional()
                .description(description),
        );
        self
    }

    pub fn optional_number_with_default(
        mut self,
        name: &str,
        description: &str,
        default: f64,
    ) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::number(name)
                .optional()
                .computed()
                .description(description)
                .default(Box::new(StaticNumber::new(default))),
        );
        self
    }

    pub fn optional_list(
        mut self,
        name: &str,
        description: &str,
        element_type: AttributeType,
    ) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::list(name, element_type)
                .optional()
                .description(description),
        );
        self
    }

    pub fn optional_map(
        mut self,
        name: &str,
        description: &str,
        element_type: AttributeType,
    ) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::map(name, element_type)
                .optional()
                .description(description),
        );
        self
    }

    pub fn computed_string(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::string(name)
                .computed()
                .description(description),
        );
        self
    }

    pub fn id_attribute(mut self) -> Self {
        self.builder = self.builder.attribute(
            "id",
            AttributeBuilder::string("id")
                .computed()
                .description("Unique identifier for the resource"),
        );
        self
    }

    pub fn with_timestamps(mut self) -> Self {
        self.builder = self
            .builder
            .attribute(
                "created_at",
                AttributeBuilder::string("created_at")
                    .computed()
                    .description("Timestamp when the resource was created")
                    .plan_modifier(Box::new(UseStateForUnknown)),
            )
            .attribute(
                "updated_at",
                AttributeBuilder::string("updated_at")
                    .computed()
                    .description("Timestamp when the resource was last updated"),
            );
        self
    }

    pub fn custom_attribute<F>(mut self, name: &str, f: F) -> Self
    where
        F: FnOnce(AttributeBuilder) -> AttributeBuilder,
    {
        let builder = AttributeBuilder::string(name);
        self.builder = self.builder.attribute(name, f(builder));
        self
    }

    pub fn build(self) -> ResourceSchema {
        self.builder.build_resource(self.version)
    }
}

pub struct FluentDataSourceSchemaBuilder {
    builder: SchemaBuilder,
    version: i64,
}

#[allow(clippy::derivable_impls)]
impl Default for FluentDataSourceSchemaBuilder {
    fn default() -> Self {
        Self {
            builder: SchemaBuilder::new(),
            version: 0,
        }
    }
}

impl FluentDataSourceSchemaBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(mut self, version: i64) -> Self {
        self.version = version;
        self
    }

    pub fn required_string(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::string(name)
                .required()
                .description(description),
        );
        self
    }

    pub fn computed_string(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::string(name)
                .computed()
                .description(description),
        );
        self
    }

    pub fn computed_number(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::number(name)
                .computed()
                .description(description),
        );
        self
    }

    pub fn computed_bool(mut self, name: &str, description: &str) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::bool(name)
                .computed()
                .description(description),
        );
        self
    }

    pub fn computed_list(
        mut self,
        name: &str,
        description: &str,
        element_type: AttributeType,
    ) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::list(name, element_type)
                .computed()
                .description(description),
        );
        self
    }

    pub fn computed_map(
        mut self,
        name: &str,
        description: &str,
        element_type: AttributeType,
    ) -> Self {
        self.builder = self.builder.attribute(
            name,
            AttributeBuilder::map(name, element_type)
                .computed()
                .description(description),
        );
        self
    }

    pub fn id_attribute(mut self) -> Self {
        self.builder = self.builder.attribute(
            "id",
            AttributeBuilder::string("id")
                .computed()
                .description("Unique identifier for the data source"),
        );
        self
    }

    pub fn build(self) -> DataSourceSchema {
        self.builder.build_data_source(self.version)
    }
}

impl ResourceSchema {
    pub fn builder() -> FluentResourceSchemaBuilder {
        FluentResourceSchemaBuilder::new()
    }
}

impl DataSourceSchema {
    pub fn builder() -> FluentDataSourceSchemaBuilder {
        FluentDataSourceSchemaBuilder::new()
    }
}

pub struct StateBuilder {
    values: HashMap<String, crate::Dynamic>,
}

impl StateBuilder {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn from_config(config: &crate::Config) -> Self {
        Self {
            values: config.values.clone(),
        }
    }

    pub fn string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values
            .insert(key.into(), crate::Dynamic::String(value.into()));
        self
    }

    pub fn bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.values.insert(key.into(), crate::Dynamic::Bool(value));
        self
    }

    pub fn number(mut self, key: impl Into<String>, value: f64) -> Self {
        self.values
            .insert(key.into(), crate::Dynamic::Number(value));
        self
    }

    pub fn list(mut self, key: impl Into<String>, value: Vec<crate::Dynamic>) -> Self {
        self.values.insert(key.into(), crate::Dynamic::List(value));
        self
    }

    pub fn map(mut self, key: impl Into<String>, value: HashMap<String, crate::Dynamic>) -> Self {
        self.values.insert(key.into(), crate::Dynamic::Map(value));
        self
    }

    pub fn build(self) -> crate::State {
        crate::State {
            values: self.values,
        }
    }
}

impl Default for StateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn fluent_resource_schema_builder_creates_schema() {
        let schema = FluentResourceSchemaBuilder::new()
            .version(1)
            .required_string("name", "Resource name")
            .optional_bool("enabled", "Whether the resource is enabled")
            .id_attribute()
            .build();

        assert_eq!(schema.version, 1);
        assert_eq!(schema.attributes.len(), 3);
        assert!(schema.attributes.contains_key("name"));
        assert!(schema.attributes.contains_key("enabled"));
        assert!(schema.attributes.contains_key("id"));

        let name_attr = schema.attributes.get("name").unwrap();
        assert!(name_attr.required);
        assert!(!name_attr.optional);
        assert_eq!(name_attr.description, "Resource name");

        let enabled_attr = schema.attributes.get("enabled").unwrap();
        assert!(!enabled_attr.required);
        assert!(enabled_attr.optional);
        assert_eq!(enabled_attr.description, "Whether the resource is enabled");

        let id_attr = schema.attributes.get("id").unwrap();
        assert!(id_attr.computed);
        assert!(!id_attr.required);
    }

    #[test]
    fn fluent_builder_with_defaults() {
        let schema = FluentResourceSchemaBuilder::new()
            .optional_string_with_default("region", "Default region", "us-east-1")
            .optional_bool_with_default("auto_create", "Auto create resources", true)
            .optional_number_with_default("timeout", "Timeout in seconds", 30.0)
            .build();

        assert_eq!(schema.attributes.len(), 3);

        let region_attr = schema.attributes.get("region").unwrap();
        assert!(region_attr.optional);
        assert!(region_attr.computed);
        assert!(region_attr.default.is_some());

        let auto_create_attr = schema.attributes.get("auto_create").unwrap();
        assert!(auto_create_attr.optional);
        assert!(auto_create_attr.computed);
        assert!(auto_create_attr.default.is_some());

        let timeout_attr = schema.attributes.get("timeout").unwrap();
        assert!(timeout_attr.optional);
        assert!(timeout_attr.computed);
        assert!(timeout_attr.default.is_some());
    }

    #[test]
    fn fluent_builder_with_sensitive_fields() {
        let schema = FluentResourceSchemaBuilder::new()
            .required_sensitive_string("api_key", "API Key for authentication")
            .build();

        let api_key_attr = schema.attributes.get("api_key").unwrap();
        assert!(api_key_attr.required);
        assert!(api_key_attr.sensitive);
    }

    #[test]
    fn fluent_builder_with_timestamps() {
        let schema = FluentResourceSchemaBuilder::new().with_timestamps().build();

        assert!(schema.attributes.contains_key("created_at"));
        assert!(schema.attributes.contains_key("updated_at"));

        let created_at = schema.attributes.get("created_at").unwrap();
        assert!(created_at.computed);
        assert!(!created_at.plan_modifiers.is_empty());
    }

    #[test]
    fn fluent_data_source_schema_builder() {
        let schema = FluentDataSourceSchemaBuilder::new()
            .version(2)
            .required_string("filter", "Filter criteria")
            .computed_string("result", "Query result")
            .computed_number("count", "Number of results")
            .computed_bool("found", "Whether results were found")
            .id_attribute()
            .build();

        assert_eq!(schema.version, 2);
        assert_eq!(schema.attributes.len(), 5);

        let filter_attr = schema.attributes.get("filter").unwrap();
        assert!(filter_attr.required);

        let result_attr = schema.attributes.get("result").unwrap();
        assert!(result_attr.computed);

        let count_attr = schema.attributes.get("count").unwrap();
        assert!(count_attr.computed);

        let found_attr = schema.attributes.get("found").unwrap();
        assert!(found_attr.computed);
    }

    #[test]
    fn state_builder_from_config() {
        let mut config = crate::Config::new();
        config.values.insert(
            "name".to_string(),
            crate::Dynamic::String("test".to_string()),
        );
        config
            .values
            .insert("enabled".to_string(), crate::Dynamic::Bool(true));

        let state = StateBuilder::from_config(&config)
            .string("id", "test-123")
            .build();

        assert_eq!(state.values.len(), 3);
        assert_eq!(state.get_string("name").unwrap(), "test");
        assert_eq!(state.get_bool("enabled"), Some(true));
        assert_eq!(state.get_string("id").unwrap(), "test-123");
    }

    #[test]
    fn state_builder_from_scratch() {
        let state = StateBuilder::new()
            .string("id", "resource-456")
            .string("name", "My Resource")
            .bool("enabled", true)
            .number("timeout", 30.0)
            .list(
                "tags",
                vec![
                    crate::Dynamic::String("tag1".to_string()),
                    crate::Dynamic::String("tag2".to_string()),
                ],
            )
            .build();

        assert_eq!(state.values.len(), 5);
        assert_eq!(state.get_string("id").unwrap(), "resource-456");
        assert_eq!(state.get_string("name").unwrap(), "My Resource");
        assert_eq!(state.get_bool("enabled"), Some(true));
        assert_eq!(state.get_number("timeout"), Some(30.0));

        let tags = state.values.get("tags").unwrap().as_list().unwrap();
        assert_eq!(tags.len(), 2);
    }
}
