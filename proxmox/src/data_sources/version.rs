use crate::api::Client;
use std::collections::HashMap;
use tfplug::provider::DataSourceSchema;
use tfplug::{AttributeBuilder, Config, DataSource, Diagnostics, Dynamic, SchemaBuilder, State};

pub struct VersionDataSource {
    client: Client,
}

impl VersionDataSource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn schema_static() -> DataSourceSchema {
        SchemaBuilder::new()
            .attribute("id", AttributeBuilder::string("id").computed())
            .attribute(
                "version",
                AttributeBuilder::string("version")
                    .computed()
                    .description("Proxmox version"),
            )
            .attribute(
                "release",
                AttributeBuilder::string("release")
                    .computed()
                    .description("Proxmox release"),
            )
            .attribute(
                "repoid",
                AttributeBuilder::string("repoid")
                    .computed()
                    .description("Repository ID"),
            )
            .build_data_source(0)
    }
}

impl DataSource for VersionDataSource {
    fn schema(&self) -> DataSourceSchema {
        Self::schema_static()
    }

    fn read(&self, _config: Config) -> tfplug::Result<(State, Diagnostics)> {
        let diags = Diagnostics::new();

        // Clone the client to move into the async block
        let client = self.client.clone();

        // Use spawn_blocking to run the async operation in a separate thread
        let version_info = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async move { client.get_version().await })
        })
        .join()
        .expect("Thread panicked")
        .map_err(|e| format!("Failed to get version: {}", e))?;

        let mut values = HashMap::new();
        values.insert(
            "id".to_string(),
            Dynamic::String("proxmox_version".to_string()),
        );
        values.insert("version".to_string(), Dynamic::String(version_info.version));
        values.insert("release".to_string(), Dynamic::String(version_info.release));
        values.insert("repoid".to_string(), Dynamic::String(version_info.repoid));

        Ok((State { values }, diags))
    }
}
