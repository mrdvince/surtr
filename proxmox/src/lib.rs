pub mod api;
pub mod data_sources;

use tfplug::{Config, DataSource, Diagnostics, Provider, Resource};
use std::collections::HashMap;

pub struct ProxmoxProvider {
    client: Option<api::Client>,
}

impl ProxmoxProvider {
    pub fn new() -> Self {
        Self { client: None }
    }
}

impl Provider for ProxmoxProvider {
    fn configure(&mut self, config: Config) -> tfplug::Result<Diagnostics> {
        let diags = Diagnostics::new();

        let endpoint = config
            .values
            .get("endpoint")
            .and_then(|v| v.as_string())
            .ok_or("endpoint is required")?;

        let api_token = config
            .values
            .get("api_token")
            .and_then(|v| v.as_string())
            .ok_or("api_token is required")?;

        let insecure = config
            .values
            .get("insecure")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        self.client = Some(api::Client::new(endpoint.clone(), api_token.clone(), insecure)?);

        Ok(diags)
    }

    fn get_schema(&self) -> HashMap<String, tfplug::provider::DataSourceSchema> {
        let mut schemas = HashMap::new();
        
        // Schema should be available without client
        schemas.insert("proxmox_version".to_string(), data_sources::VersionDataSource::schema_static());
        
        schemas
    }

    fn get_resources(&self) -> HashMap<String, Box<dyn Resource>> {
        HashMap::new()
    }

    fn get_data_sources(&self) -> HashMap<String, Box<dyn DataSource>> {
        let mut sources = HashMap::new();
        
        // Return empty map if client is not initialized
        // The framework should configure the provider before calling read on data sources
        if let Some(client) = &self.client {
            sources.insert(
                "proxmox_version".to_string(),
                Box::new(data_sources::VersionDataSource::new(client.clone())) as Box<dyn DataSource>,
            );
        }
        
        sources
    }
}