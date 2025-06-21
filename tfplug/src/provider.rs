use crate::types::{Config, Diagnostics, State};
use crate::Result;
use std::collections::HashMap;

pub trait Provider: Send + Sync {
    fn configure(&mut self, config: Config) -> Result<Diagnostics>;

    fn get_schema(&self) -> HashMap<String, DataSourceSchema>;

    fn get_resources(&self) -> HashMap<String, Box<dyn Resource>>;

    fn get_data_sources(&self) -> HashMap<String, Box<dyn DataSource>>;
}

pub trait Resource: Send + Sync {
    fn schema(&self) -> ResourceSchema;

    fn create(&self, config: Config) -> Result<(State, Diagnostics)>;

    fn read(&self, state: State) -> Result<(Option<State>, Diagnostics)>;

    fn update(&self, state: State, config: Config) -> Result<(State, Diagnostics)>;

    fn delete(&self, state: State) -> Result<Diagnostics>;
}

pub trait DataSource: Send + Sync {
    fn schema(&self) -> DataSourceSchema;

    fn read(&self, config: Config) -> Result<(State, Diagnostics)>;
}

#[derive(Debug, Clone)]
pub struct DataSourceSchema {
    pub version: i64,
    pub attributes: HashMap<String, Attribute>,
}

#[derive(Debug, Clone)]
pub struct ResourceSchema {
    pub version: i64,
    pub attributes: HashMap<String, Attribute>,
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub r#type: Vec<u8>,
    pub description: String,
    pub required: bool,
    pub optional: bool,
    pub computed: bool,
    pub sensitive: bool,
}
