pub mod attribute_type;
pub mod builders;
pub mod context;
pub mod defaults;
pub mod grpc;
pub mod plan_modifier;
pub mod proto;
pub mod provider;
pub mod request;
pub mod schema;
pub mod types;
pub mod validator;

pub use builders::{FluentDataSourceSchemaBuilder, FluentResourceSchemaBuilder, StateBuilder};
pub use defaults::{
    Default, DefaultRequest, DefaultResponse, StaticBool, StaticNumber, StaticString,
};
pub use grpc::ProviderServer;
pub use plan_modifier::{
    PlanModifier, PlanModifyRequest, PlanModifyResponse, RequiresReplaceIfChanged,
    UseStateForUnknown,
};
pub use provider::{DataSourceV2, ProviderV2, ResourceV2};
pub use schema::{AttributeBuilder, SchemaBuilder};
pub use types::{Config, Diagnostics, Dynamic, State};

use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use rmp_serde::{from_slice, to_vec_named};
    use std::collections::HashMap;

    #[test]
    fn dynamic_serialization_as_map() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), Dynamic::String("value".to_string()));
        map.insert("number".to_string(), Dynamic::Number(42.0));

        let dynamic = Dynamic::Map(map);

        let encoded = to_vec_named(&dynamic).unwrap();
        let decoded: Dynamic = from_slice(&encoded).unwrap();

        match decoded {
            Dynamic::Map(m) => {
                assert_eq!(m.get("key").unwrap().as_string().unwrap(), "value");
                assert_eq!(m.get("number").unwrap().as_number().unwrap(), 42.0);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn state_serialization_preserves_field_names() {
        let mut values = HashMap::new();
        values.insert("id".to_string(), Dynamic::String("test-123".to_string()));
        values.insert("enabled".to_string(), Dynamic::Bool(true));

        let state = State { values };

        let encoded = to_vec_named(&state).unwrap();
        let decoded: State = from_slice(&encoded).unwrap();

        assert_eq!(
            decoded.values.get("id").unwrap().as_string().unwrap(),
            "test-123"
        );
        assert_eq!(
            decoded.values.get("enabled").unwrap().as_bool().unwrap(),
            true
        );
    }

    #[test]
    fn dynamic_unknown_serialization() {
        let unknown = Dynamic::Unknown;
        let encoded = rmp_serde::to_vec_named(&unknown).unwrap();
        let decoded: Dynamic = rmp_serde::from_slice(&encoded).unwrap();

        eprintln!("Original: {:?}", unknown);
        eprintln!("Decoded: {:?}", decoded);
        eprintln!("Encoded bytes: {:?}", encoded);
    }

    #[test]
    fn dynamic_null_handling() {
        let dynamic = Dynamic::Null;
        let encoded = to_vec_named(&dynamic).unwrap();
        let decoded: Dynamic = from_slice(&encoded).unwrap();

        assert!(matches!(decoded, Dynamic::Null));
    }

    #[test]
    fn nested_dynamic_structures() {
        let mut inner = HashMap::new();
        inner.insert("nested".to_string(), Dynamic::String("value".to_string()));

        let mut outer = HashMap::new();
        outer.insert(
            "list".to_string(),
            Dynamic::List(vec![Dynamic::Number(1.0), Dynamic::Number(2.0)]),
        );
        outer.insert("map".to_string(), Dynamic::Map(inner));

        let dynamic = Dynamic::Map(outer);

        let encoded = to_vec_named(&dynamic).unwrap();
        let decoded: Dynamic = from_slice(&encoded).unwrap();

        match decoded {
            Dynamic::Map(m) => {
                match m.get("list").unwrap() {
                    Dynamic::List(l) => assert_eq!(l.len(), 2),
                    _ => panic!("Expected List"),
                }
                match m.get("map").unwrap() {
                    Dynamic::Map(inner_map) => {
                        assert_eq!(
                            inner_map.get("nested").unwrap().as_string().unwrap(),
                            "value"
                        );
                    }
                    _ => panic!("Expected Map"),
                }
            }
            _ => panic!("Expected Map"),
        }
    }
}
