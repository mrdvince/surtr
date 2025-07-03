use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeType {
    String,
    Number,
    Bool,
    List(Box<AttributeType>),
    Set(Box<AttributeType>),
    Map(Box<AttributeType>),
    Object(HashMap<String, AttributeType>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_type_string_creates_correct_type() {
        let attr_type = AttributeType::String;
        assert!(matches!(attr_type, AttributeType::String));
    }

    #[test]
    fn attribute_type_number_creates_correct_type() {
        let attr_type = AttributeType::Number;
        assert!(matches!(attr_type, AttributeType::Number));
    }

    #[test]
    fn attribute_type_bool_creates_correct_type() {
        let attr_type = AttributeType::Bool;
        assert!(matches!(attr_type, AttributeType::Bool));
    }

    #[test]
    fn attribute_type_list_contains_element_type() {
        let attr_type = AttributeType::List(Box::new(AttributeType::String));

        match attr_type {
            AttributeType::List(elem_type) => {
                assert!(matches!(*elem_type, AttributeType::String));
            }
            _ => panic!("Expected List type"),
        }
    }

    #[test]
    fn attribute_type_set_contains_element_type() {
        let attr_type = AttributeType::Set(Box::new(AttributeType::Number));

        match attr_type {
            AttributeType::Set(elem_type) => {
                assert!(matches!(*elem_type, AttributeType::Number));
            }
            _ => panic!("Expected Set type"),
        }
    }

    #[test]
    fn attribute_type_map_contains_element_type() {
        let attr_type = AttributeType::Map(Box::new(AttributeType::Bool));

        match attr_type {
            AttributeType::Map(elem_type) => {
                assert!(matches!(*elem_type, AttributeType::Bool));
            }
            _ => panic!("Expected Map type"),
        }
    }

    #[test]
    fn attribute_type_object_contains_attributes() {
        use std::collections::HashMap;

        let mut attrs = HashMap::new();
        attrs.insert("name".to_string(), AttributeType::String);
        attrs.insert("age".to_string(), AttributeType::Number);

        let attr_type = AttributeType::Object(attrs.clone());

        match attr_type {
            AttributeType::Object(obj_attrs) => {
                assert_eq!(obj_attrs.len(), 2);
                assert!(matches!(obj_attrs.get("name"), Some(AttributeType::String)));
                assert!(matches!(obj_attrs.get("age"), Some(AttributeType::Number)));
            }
            _ => panic!("Expected Object type"),
        }
    }

    #[test]
    fn nested_collection_types_work() {
        let attr_type = AttributeType::List(Box::new(AttributeType::Map(Box::new(
            AttributeType::String,
        ))));

        match attr_type {
            AttributeType::List(elem_type) => match elem_type.as_ref() {
                AttributeType::Map(map_elem_type) => {
                    assert!(matches!(**map_elem_type, AttributeType::String));
                }
                _ => panic!("Expected Map inside List"),
            },
            _ => panic!("Expected List type"),
        }
    }
}
