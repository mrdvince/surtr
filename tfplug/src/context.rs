use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Context {
    values: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            values: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_value<T: Any + Send + Sync + 'static>(self, value: T) -> Self {
        self.values
            .write()
            .expect("Context lock poisoned")
            .insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.values
            .read()
            .expect("Context lock poisoned")
            .get(&TypeId::of::<T>())
            .and_then(|v| v.clone().downcast().ok())
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestValue(String);

    #[test]
    fn context_stores_and_retrieves_values() {
        let ctx = Context::new().with_value(TestValue("hello".to_string()));

        let value = ctx.get::<TestValue>().unwrap();
        assert_eq!(value.0, "hello");
    }

    #[test]
    fn context_returns_none_for_missing_values() {
        let ctx = Context::new();
        assert!(ctx.get::<TestValue>().is_none());
    }

    #[test]
    fn context_can_store_multiple_types() {
        let ctx = Context::new()
            .with_value(TestValue("test".to_string()))
            .with_value(42u32)
            .with_value("string value".to_string());

        assert_eq!(ctx.get::<TestValue>().unwrap().0, "test");
        assert_eq!(*ctx.get::<u32>().unwrap(), 42);
        assert_eq!(ctx.get::<String>().unwrap().as_str(), "string value");
    }
}
