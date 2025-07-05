//! Schema types and builders for tfplug
//!
//! This module provides the schema system for defining resource and data source
//! schemas, including attribute types, blocks, and validation.

use crate::types::{AttributePath, Diagnostic};
use std::collections::HashMap;

/// AttributeType defines the type system for Terraform attributes
/// This must match Terraform's type system exactly
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeType {
    String,
    Number, // Always f64
    Bool,
    List(Box<AttributeType>),               // Ordered, allows duplicates
    Set(Box<AttributeType>),                // Unordered, no duplicates
    Map(Box<AttributeType>),                // String keys only
    Object(HashMap<String, AttributeType>), // Fixed structure
}

/// Schema is returned by providers/resources/data sources
/// Version is used for state migration
#[derive(Debug, Clone)]
pub struct Schema {
    pub version: i64, // Increment when schema changes require migration
    pub block: Block, // Root block containing all attributes
}

/// Block represents a configuration block
#[derive(Debug, Clone)]
pub struct Block {
    pub version: i64,
    pub attributes: Vec<Attribute>,
    pub block_types: Vec<NestedBlock>,
    pub description: String,
    pub description_kind: StringKind,
    pub deprecated: bool,
}

/// Attribute represents a single configuration attribute
pub struct Attribute {
    pub name: String,
    pub r#type: AttributeType,
    pub description: String,
    pub required: bool,
    pub optional: bool,
    pub computed: bool,
    pub sensitive: bool,
    pub validators: Vec<Box<dyn Validator>>,
    pub plan_modifiers: Vec<Box<dyn PlanModifier>>,
    pub default: Option<Box<dyn Default>>,
    pub nested_type: Option<NestedType>,
    pub deprecated: bool,
}

// Manual Debug implementation since validators/modifiers don't implement Debug
impl std::fmt::Debug for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.name)
            .field("type", &self.r#type)
            .field("description", &self.description)
            .field("required", &self.required)
            .field("optional", &self.optional)
            .field("computed", &self.computed)
            .field("sensitive", &self.sensitive)
            .field(
                "validators",
                &format!("{} validators", self.validators.len()),
            )
            .field(
                "plan_modifiers",
                &format!("{} plan modifiers", self.plan_modifiers.len()),
            )
            .field("default", &self.default.is_some())
            .field("nested_type", &self.nested_type)
            .field("deprecated", &self.deprecated)
            .finish()
    }
}

// Manual Clone implementation
impl Clone for Attribute {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            r#type: self.r#type.clone(),
            description: self.description.clone(),
            required: self.required,
            optional: self.optional,
            computed: self.computed,
            sensitive: self.sensitive,
            validators: vec![],
            plan_modifiers: vec![],
            default: None,
            nested_type: self.nested_type.clone(),
            deprecated: self.deprecated,
        }
    }
}

/// NestedBlock represents a nested configuration block
#[derive(Debug, Clone)]
pub struct NestedBlock {
    pub type_name: String,
    pub block: Block,
    pub nesting: NestingMode,
    pub min_items: i64,
    pub max_items: i64,
}

/// NestingMode defines how nested blocks are structured
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NestingMode {
    Invalid,
    Single,
    List,
    Set,
    Map,
    Group,
}

/// NestedType for attributes with nested structures
#[derive(Debug, Clone)]
pub struct NestedType {
    pub attributes: Vec<Attribute>,
    pub nesting: ObjectNestingMode,
}

/// ObjectNestingMode for nested attribute objects
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectNestingMode {
    Invalid,
    Single,
    List,
    Set,
    Map,
}

/// StringKind represents the format of string values
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StringKind {
    Plain,
    Markdown,
}

/// Validator performs validation on attribute values during planning
/// Implement this for custom validation logic
pub trait Validator: Send + Sync {
    /// Human-readable description
    fn description(&self) -> String;
    /// Perform validation
    fn validate(&self, request: ValidatorRequest) -> ValidatorResponse;
}

/// Request for validators
pub struct ValidatorRequest {
    pub config_value: crate::types::DynamicValue,
    pub path: AttributePath,
}

/// Response from validators
pub struct ValidatorResponse {
    pub diagnostics: Vec<Diagnostic>,
}

/// PlanModifier modifies planned values during planning
/// Common uses: RequiresReplace, UseStateForUnknown
pub trait PlanModifier: Send + Sync {
    /// Human-readable description
    fn description(&self) -> String;
    /// Modify the planned value
    fn modify(&self, request: PlanModifierRequest) -> PlanModifierResponse;
}

/// Request for plan modifiers
pub struct PlanModifierRequest {
    pub config_value: crate::types::DynamicValue,
    pub state_value: crate::types::DynamicValue,
    pub plan_value: crate::types::DynamicValue,
    pub path: AttributePath,
}

/// Response from plan modifiers
pub struct PlanModifierResponse {
    pub plan_value: crate::types::DynamicValue,
    pub requires_replace: bool,
    pub diagnostics: Vec<Diagnostic>,
}

/// Default provides default values for optional attributes
/// Called when attribute is not set in configuration
pub trait Default: Send + Sync {
    /// Human-readable description
    fn description(&self) -> String;
    /// Provide default value
    fn default_value(&self, request: DefaultRequest) -> DefaultResponse;
}

/// Request for default values
pub struct DefaultRequest {
    pub path: AttributePath,
}

/// Response with default value
pub struct DefaultResponse {
    pub value: crate::types::DynamicValue,
}

/// AttributeBuilder provides fluent API for building attributes
/// ALWAYS use this instead of constructing Attribute directly
pub struct AttributeBuilder {
    attribute: Attribute,
}

impl AttributeBuilder {
    /// Create a new attribute builder
    pub fn new(name: &str, type_: AttributeType) -> Self {
        Self {
            attribute: Attribute {
                name: name.to_string(),
                r#type: type_,
                description: String::new(),
                required: false,
                optional: false,
                computed: false,
                sensitive: false,
                validators: Vec::new(),
                plan_modifiers: Vec::new(),
                default: None,
                nested_type: None,
                deprecated: false,
            },
        }
    }

    /// Set description
    pub fn description(mut self, desc: &str) -> Self {
        self.attribute.description = desc.to_string();
        self
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.attribute.required = true;
        self.attribute.optional = false;
        self
    }

    /// Mark as optional
    pub fn optional(mut self) -> Self {
        self.attribute.optional = true;
        self.attribute.required = false;
        self
    }

    /// Mark as computed
    pub fn computed(mut self) -> Self {
        self.attribute.computed = true;
        self
    }

    /// Mark as sensitive (hidden)
    pub fn sensitive(mut self) -> Self {
        self.attribute.sensitive = true;
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.attribute.deprecated = true;
        self
    }

    /// Add validator
    pub fn validator(mut self, validator: Box<dyn Validator>) -> Self {
        self.attribute.validators.push(validator);
        self
    }

    /// Add plan modifier
    pub fn plan_modifier(mut self, modifier: Box<dyn PlanModifier>) -> Self {
        self.attribute.plan_modifiers.push(modifier);
        self
    }

    /// Set default
    pub fn default(mut self, default: Box<dyn Default>) -> Self {
        self.attribute.default = Some(default);
        self
    }

    /// Set nested type
    pub fn nested_type(mut self, nested: NestedType) -> Self {
        self.attribute.nested_type = Some(nested);
        self
    }

    /// Finalize the attribute
    pub fn build(self) -> Attribute {
        self.attribute
    }
}

/// SchemaBuilder provides fluent API for building schemas
/// ALWAYS use this for consistency
pub struct SchemaBuilder {
    schema: Schema,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            schema: Schema {
                version: 0,
                block: Block {
                    version: 0,
                    attributes: Vec::new(),
                    block_types: Vec::new(),
                    description: String::new(),
                    description_kind: StringKind::Plain,
                    deprecated: false,
                },
            },
        }
    }

    /// Set schema version
    pub fn version(mut self, version: i64) -> Self {
        self.schema.version = version;
        self.schema.block.version = version;
        self
    }

    /// Add attribute
    pub fn attribute(mut self, attr: Attribute) -> Self {
        self.schema.block.attributes.push(attr);
        self
    }

    /// Add nested block
    pub fn block(mut self, block: NestedBlock) -> Self {
        self.schema.block.block_types.push(block);
        self
    }

    /// Set description
    pub fn description(mut self, desc: &str) -> Self {
        self.schema.block.description = desc.to_string();
        self
    }

    /// Set description kind
    pub fn description_kind(mut self, kind: StringKind) -> Self {
        self.schema.block.description_kind = kind;
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.schema.block.deprecated = true;
        self
    }

    /// Finalize the schema
    pub fn build(self) -> Schema {
        self.schema
    }
}

impl std::default::Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_builder_creates_required_string() {
        let attr = AttributeBuilder::new("name", AttributeType::String)
            .description("The name of the resource")
            .required()
            .build();

        assert_eq!(attr.name, "name");
        assert!(matches!(attr.r#type, AttributeType::String));
        assert!(attr.required);
        assert!(!attr.optional);
        assert_eq!(attr.description, "The name of the resource");
    }

    #[test]
    fn schema_builder_creates_schema_with_attributes() {
        let schema = SchemaBuilder::new()
            .version(1)
            .description("Test resource schema")
            .attribute(
                AttributeBuilder::new("id", AttributeType::String)
                    .computed()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("name", AttributeType::String)
                    .required()
                    .build(),
            )
            .build();

        assert_eq!(schema.version, 1);
        assert_eq!(schema.block.attributes.len(), 2);
        assert_eq!(schema.block.description, "Test resource schema");
    }

    #[test]
    fn nested_attribute_type() {
        let object_type = AttributeType::Object(HashMap::from([
            ("host".to_string(), AttributeType::String),
            ("port".to_string(), AttributeType::Number),
        ]));

        let attr = AttributeBuilder::new("config", object_type)
            .optional()
            .build();

        assert!(attr.optional);
        if let AttributeType::Object(fields) = &attr.r#type {
            assert_eq!(fields.len(), 2);
            assert!(matches!(fields.get("host"), Some(AttributeType::String)));
            assert!(matches!(fields.get("port"), Some(AttributeType::Number)));
        } else {
            panic!("Expected Object type");
        }
    }
}
