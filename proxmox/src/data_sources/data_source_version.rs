//! Version data source implementation

use async_trait::async_trait;
use tfplug::context::Context;
use tfplug::data_source::{
    ConfigureDataSourceRequest, ConfigureDataSourceResponse, DataSource, DataSourceMetadataRequest,
    DataSourceMetadataResponse, DataSourceSchemaRequest, DataSourceSchemaResponse,
    DataSourceWithConfigure, ReadDataSourceRequest, ReadDataSourceResponse,
    ValidateDataSourceConfigRequest, ValidateDataSourceConfigResponse,
};
use tfplug::schema::{AttributeBuilder, AttributeType, SchemaBuilder};
use tfplug::types::{AttributePath, Diagnostic, DynamicValue};

#[derive(Default)]
pub struct VersionDataSource {
    provider_data: Option<crate::ProxmoxProviderData>,
}

impl VersionDataSource {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DataSource for VersionDataSource {
    fn type_name(&self) -> &str {
        "proxmox_version"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: DataSourceMetadataRequest,
    ) -> DataSourceMetadataResponse {
        DataSourceMetadataResponse {
            type_name: self.type_name().to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: DataSourceSchemaRequest,
    ) -> DataSourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .version(0)
            .description("Gets the Proxmox VE version information")
            .attribute(
                AttributeBuilder::new("id", AttributeType::String)
                    .description("The data source ID")
                    .computed()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("version", AttributeType::String)
                    .description("The Proxmox VE version")
                    .computed()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("release", AttributeType::String)
                    .description("The Proxmox VE release")
                    .computed()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("repoid", AttributeType::String)
                    .description("The Proxmox VE repository ID")
                    .computed()
                    .build(),
            )
            .build();

        DataSourceSchemaResponse {
            schema,
            diagnostics: vec![],
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        _request: ValidateDataSourceConfigRequest,
    ) -> ValidateDataSourceConfigResponse {
        ValidateDataSourceConfigResponse {
            diagnostics: vec![],
        }
    }

    async fn read(&self, _ctx: Context, _request: ReadDataSourceRequest) -> ReadDataSourceResponse {
        let mut diagnostics = vec![];

        tracing::debug!(
            "Reading version data source, provider_data: {:?}",
            self.provider_data.is_some()
        );

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Provider data was not properly configured",
                ));
                return ReadDataSourceResponse {
                    state: DynamicValue::null(),
                    diagnostics,
                    deferred: None,
                };
            }
        };

        match provider_data.client.get_version().await {
            Ok(version_info) => {
                let mut state = DynamicValue::null();
                let _ = state.set_string(&AttributePath::new("id"), "proxmox-version".to_string());
                let _ = state.set_string(&AttributePath::new("version"), version_info.version);
                let _ = state.set_string(&AttributePath::new("release"), version_info.release);
                let _ = state.set_string(&AttributePath::new("repoid"), version_info.repoid);

                ReadDataSourceResponse {
                    state,
                    diagnostics,
                    deferred: None,
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Failed to get version information",
                    format!("API error: {}", e),
                ));
                ReadDataSourceResponse {
                    state: DynamicValue::null(),
                    diagnostics,
                    deferred: None,
                }
            }
        }
    }
}

#[async_trait]
impl DataSourceWithConfigure for VersionDataSource {
    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureDataSourceRequest,
    ) -> ConfigureDataSourceResponse {
        let mut diagnostics = vec![];

        tracing::debug!(
            "Configuring version data source, provider_data provided: {:?}",
            request.provider_data.is_some()
        );

        if let Some(data) = request.provider_data {
            tracing::debug!("Attempting to downcast provider data");
            if let Some(provider_data) = data.downcast_ref::<crate::ProxmoxProviderData>() {
                self.provider_data = Some(provider_data.clone());
                tracing::debug!("Successfully configured version data source with provider data");
            } else {
                tracing::error!("Failed to downcast provider data to ProxmoxProviderData");
                tracing::error!("Provider data type id: {:?}", data.type_id());
                diagnostics.push(Diagnostic::error(
                    "Invalid provider data",
                    "Failed to extract ProxmoxProviderData from provider data",
                ));
            }
        } else {
            tracing::warn!("No provider data provided to version data source");
            diagnostics.push(Diagnostic::error(
                "No provider data",
                "No provider data was provided to the data source",
            ));
        }

        ConfigureDataSourceResponse { diagnostics }
    }
}
