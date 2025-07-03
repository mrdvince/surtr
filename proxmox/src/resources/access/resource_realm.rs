//! Realm resource implementation

use async_trait::async_trait;
use tfplug::context::Context;
use tfplug::resource::{
    ConfigureResourceRequest, ConfigureResourceResponse, CreateResourceRequest,
    CreateResourceResponse, DeleteResourceRequest, DeleteResourceResponse, ReadResourceRequest,
    ReadResourceResponse, Resource, ResourceMetadataRequest, ResourceMetadataResponse,
    ResourceSchemaRequest, ResourceSchemaResponse, ResourceWithConfigure, UpdateResourceRequest,
    UpdateResourceResponse, ValidateResourceConfigRequest, ValidateResourceConfigResponse,
};
use tfplug::schema::{AttributeBuilder, AttributeType, SchemaBuilder};
use tfplug::types::{AttributePath, Diagnostic, DynamicValue};

#[derive(Default)]
pub struct RealmResource {
    provider_data: Option<crate::ProxmoxProviderData>,
}

impl RealmResource {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Resource for RealmResource {
    fn type_name(&self) -> &str {
        "proxmox_realm"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: self.type_name().to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .version(0)
            .description("Manages authentication realms in Proxmox VE")
            .attribute(
                AttributeBuilder::new("realm", AttributeType::String)
                    .description("The realm identifier")
                    .required()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("type", AttributeType::String)
                    .description("The authentication type (e.g., openid, ldap, ad, pam, pve)")
                    .required()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("issuer_url", AttributeType::String)
                    .description("OpenID Connect issuer URL")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("client_id", AttributeType::String)
                    .description("OpenID Connect client ID")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("client_key", AttributeType::String)
                    .description("OpenID Connect client secret")
                    .optional()
                    .sensitive()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("username_claim", AttributeType::String)
                    .description("OpenID claim used to generate the unique username")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("autocreate", AttributeType::Bool)
                    .description("Automatically create users if they do not exist")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("default", AttributeType::Bool)
                    .description("Use this as the default realm")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("comment", AttributeType::String)
                    .description("Description of the realm")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("groups_overwrite", AttributeType::Bool)
                    .description("Overwrite existing groups on login")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("groups_autocreate", AttributeType::Bool)
                    .description("Automatically create groups that do not exist")
                    .optional()
                    .build(),
            )
            .build();

        ResourceSchemaResponse {
            schema,
            diagnostics: vec![],
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        request: ValidateResourceConfigRequest,
    ) -> ValidateResourceConfigResponse {
        let mut diagnostics = vec![];

        // Validate realm type
        if let Ok(realm_type) = request.config.get_string(&AttributePath::new("type")) {
            let valid_types = ["openid", "ldap", "ad", "pam", "pve"];
            if !valid_types.contains(&realm_type.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "Invalid realm type",
                    format!("Realm type must be one of: {:?}", valid_types),
                ));
            }
        }

        ValidateResourceConfigResponse { diagnostics }
    }

    async fn create(
        &self,
        _ctx: Context,
        request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        let mut diagnostics = vec![];

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Provider data was not properly configured",
                ));
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                };
            }
        };

        // Extract realm configuration from request
        match self.extract_realm_config(&request.config) {
            Ok(realm_config) => {
                // Call API to create realm
                let create_request = crate::api::access::realms::CreateRealmRequest {
                    realm: realm_config.realm.clone(),
                    realm_type: realm_config.realm_type.clone(),
                    comment: realm_config.comment.clone(),
                    default: realm_config.default,
                    issuer_url: realm_config.issuer_url.clone(),
                    client_id: realm_config.client_id.clone(),
                    client_key: realm_config.client_key.clone(),
                    username_claim: realm_config.username_claim.clone(),
                    autocreate: realm_config.autocreate,
                    groups_overwrite: realm_config.groups_overwrite,
                    groups_autocreate: realm_config.groups_autocreate,
                };
                match provider_data
                    .client
                    .access()
                    .realms()
                    .create(&create_request)
                    .await
                {
                    Ok(()) => {
                        // Return the planned state with any computed values
                        CreateResourceResponse {
                            new_state: request.planned_state,
                            private: vec![],
                            diagnostics,
                        }
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            "Failed to create realm",
                            format!("API error: {}", e),
                        ));
                        CreateResourceResponse {
                            new_state: request.planned_state,
                            private: vec![],
                            diagnostics,
                        }
                    }
                }
            }
            Err(diag) => {
                diagnostics.push(diag);
                CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                }
            }
        }
    }

    async fn read(&self, _ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
        let mut diagnostics = vec![];

        // Get realm name from state
        let realm_name = match request
            .current_state
            .get_string(&AttributePath::new("realm"))
        {
            Ok(name) => name,
            Err(_) => {
                // If we can't get the realm name, the resource is probably corrupted
                return ReadResourceResponse {
                    new_state: None, // This will mark the resource for recreation
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        // Read realm from API
        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Provider data was not properly configured",
                ));
                return ReadResourceResponse {
                    new_state: Some(request.current_state),
                    private: request.private,
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        match provider_data
            .client
            .access()
            .realms()
            .get(&realm_name)
            .await
        {
            Ok(realm_config) => {
                // Update state with current values from API
                let mut new_state = request.current_state.clone();

                // Update values that might have changed
                let _ = new_state.set_string(&AttributePath::new("type"), realm_config.realm_type);
                if let Some(comment) = realm_config.comment {
                    let _ = new_state.set_string(&AttributePath::new("comment"), comment);
                }
                if let Some(default) = realm_config.default {
                    let _ = new_state.set_bool(&AttributePath::new("default"), default);
                }
                if let Some(issuer_url) = realm_config.issuer_url {
                    let _ = new_state.set_string(&AttributePath::new("issuer_url"), issuer_url);
                }
                if let Some(client_id) = realm_config.client_id {
                    let _ = new_state.set_string(&AttributePath::new("client_id"), client_id);
                }
                if let Some(username_claim) = realm_config.username_claim {
                    let _ =
                        new_state.set_string(&AttributePath::new("username_claim"), username_claim);
                }
                if let Some(autocreate) = realm_config.autocreate {
                    let _ = new_state.set_bool(&AttributePath::new("autocreate"), autocreate);
                }
                if let Some(groups_overwrite) = realm_config.groups_overwrite {
                    let _ = new_state
                        .set_bool(&AttributePath::new("groups_overwrite"), groups_overwrite);
                }
                if let Some(groups_autocreate) = realm_config.groups_autocreate {
                    let _ = new_state
                        .set_bool(&AttributePath::new("groups_autocreate"), groups_autocreate);
                }

                ReadResourceResponse {
                    new_state: Some(new_state),
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                }
            }
            Err(crate::api::ApiError::ApiError { message, .. })
                if message.contains("does not exist") =>
            {
                // Resource doesn't exist - return None to signal Terraform to create it
                ReadResourceResponse {
                    new_state: None,
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Failed to read realm",
                    format!("API error: {}", e),
                ));
                ReadResourceResponse {
                    new_state: Some(request.current_state),
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                }
            }
        }
    }

    async fn update(
        &self,
        _ctx: Context,
        request: UpdateResourceRequest,
    ) -> UpdateResourceResponse {
        let mut diagnostics = vec![];

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Provider data was not properly configured",
                ));
                return UpdateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        // Extract realm configuration from planned state
        match self.extract_realm_config(&request.config) {
            Ok(realm_config) => {
                let update_request = crate::api::access::realms::UpdateRealmRequest {
                    realm_type: realm_config.realm_type.clone(),
                    comment: realm_config.comment.clone(),
                    default: realm_config.default,
                    issuer_url: realm_config.issuer_url.clone(),
                    client_id: realm_config.client_id.clone(),
                    client_key: realm_config.client_key.clone(),
                    username_claim: realm_config.username_claim.clone(),
                    autocreate: realm_config.autocreate,
                    groups_overwrite: realm_config.groups_overwrite,
                    groups_autocreate: realm_config.groups_autocreate,
                };
                match provider_data
                    .client
                    .access()
                    .realms()
                    .update(&realm_config.realm, &update_request)
                    .await
                {
                    Ok(()) => UpdateResourceResponse {
                        new_state: request.planned_state,
                        private: vec![],
                        diagnostics,
                        new_identity: None,
                    },
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            "Failed to update realm",
                            format!("API error: {}", e),
                        ));
                        UpdateResourceResponse {
                            new_state: request.prior_state,
                            private: vec![],
                            diagnostics,
                            new_identity: None,
                        }
                    }
                }
            }
            Err(diag) => {
                diagnostics.push(diag);
                UpdateResourceResponse {
                    new_state: request.prior_state,
                    private: vec![],
                    diagnostics,
                    new_identity: None,
                }
            }
        }
    }

    async fn delete(
        &self,
        _ctx: Context,
        request: DeleteResourceRequest,
    ) -> DeleteResourceResponse {
        let mut diagnostics = vec![];

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                // If client is not configured, we can't delete but that's probably OK
                return DeleteResourceResponse { diagnostics };
            }
        };

        // Get realm name from prior state
        let realm_name = match request.prior_state.get_string(&AttributePath::new("realm")) {
            Ok(name) => name,
            Err(_) => {
                // If we can't get the realm name, we'll just consider it deleted
                return DeleteResourceResponse { diagnostics };
            }
        };

        // Call API to delete realm
        match provider_data
            .client
            .access()
            .realms()
            .delete(&realm_name)
            .await
        {
            Ok(()) => DeleteResourceResponse { diagnostics },
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Failed to delete realm",
                    format!("API error: {}", e),
                ));
                DeleteResourceResponse { diagnostics }
            }
        }
    }
}

impl RealmResource {
    /// Extract realm configuration from terraform configuration
    fn extract_realm_config(
        &self,
        config: &DynamicValue,
    ) -> Result<crate::api::access::realms::Realm, Diagnostic> {
        let realm = config
            .get_string(&AttributePath::new("realm"))
            .map_err(|_| Diagnostic::error("Missing realm", "The 'realm' attribute is required"))?;

        let realm_type = config
            .get_string(&AttributePath::new("type"))
            .map_err(|_| Diagnostic::error("Missing type", "The 'type' attribute is required"))?;

        let comment = config.get_string(&AttributePath::new("comment")).ok();
        let default = config.get_bool(&AttributePath::new("default")).ok();
        let issuer_url = config.get_string(&AttributePath::new("issuer_url")).ok();
        let client_id = config.get_string(&AttributePath::new("client_id")).ok();
        let client_key = config.get_string(&AttributePath::new("client_key")).ok();
        let username_claim = config
            .get_string(&AttributePath::new("username_claim"))
            .ok();
        let autocreate = config.get_bool(&AttributePath::new("autocreate")).ok();
        let groups_overwrite = config
            .get_bool(&AttributePath::new("groups_overwrite"))
            .ok();
        let groups_autocreate = config
            .get_bool(&AttributePath::new("groups_autocreate"))
            .ok();

        Ok(crate::api::access::realms::Realm {
            realm,
            realm_type,
            comment,
            default,
            issuer_url,
            client_id,
            client_key,
            username_claim,
            autocreate,
            groups_overwrite,
            groups_autocreate,
        })
    }
}

#[async_trait]
impl ResourceWithConfigure for RealmResource {
    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureResourceRequest,
    ) -> ConfigureResourceResponse {
        let mut diagnostics = vec![];

        if let Some(data) = request.provider_data {
            if let Some(provider_data) = data.downcast_ref::<crate::ProxmoxProviderData>() {
                self.provider_data = Some(provider_data.clone());
            } else {
                diagnostics.push(Diagnostic::error(
                    "Invalid provider data",
                    "Failed to extract ProxmoxProviderData from provider data",
                ));
            }
        } else {
            diagnostics.push(Diagnostic::error(
                "No provider data",
                "No provider data was provided to the resource",
            ));
        }

        ConfigureResourceResponse { diagnostics }
    }
}
