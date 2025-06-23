//! gRPC service implementation for ProviderV2
//!
//! This module implements the Terraform Plugin Protocol v6.9 using the factory-based
//! ProviderV2 architecture, which eliminates locks and creates resources on demand.

use crate::attribute_type::AttributeType;
use crate::context::Context;
use crate::proto::tfplugin6::{
    provider_server::{Provider as ProtoProvider, ProviderServer as ProtoProviderServer},
    *,
};
use crate::provider::ProviderV2;
use crate::request::{CreateRequest, DeleteRequest, ReadRequest, UpdateRequest};
use crate::types::{Config, Diagnostics as TfplugDiagnostics, Dynamic, State};
use crate::Result;
use rmp_serde::{decode, encode};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status};

pub struct ProviderServer<P: ProviderV2> {
    provider: Arc<RwLock<P>>,
    cert_path: PathBuf,
    key_path: PathBuf,
}

impl<P: ProviderV2 + 'static> ProviderServer<P> {
    pub fn new(provider: P, cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            provider: Arc::new(RwLock::new(provider)),
            cert_path,
            key_path,
        }
    }

    pub async fn run(self) -> Result<()> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let cert = tokio::fs::read(&self.cert_path).await?;
        let key = tokio::fs::read(&self.key_path).await?;
        let identity = Identity::from_pem(cert, key);

        let tls_config = ServerTlsConfig::new().identity(identity);

        let addr = "127.0.0.1:0";
        let listener = TcpListener::bind(addr).await?;
        let bound_addr = listener.local_addr()?;

        println!("1|6|tcp|127.0.0.1:{}|grpc", bound_addr.port());
        eprintln!(
            "DEBUG: Provider server started on port {}",
            bound_addr.port()
        );

        let stream = TcpListenerStream::new(listener);

        let service = ProviderService {
            provider: self.provider.clone(),
        };

        Server::builder()
            .tls_config(tls_config)?
            .add_service(ProtoProviderServer::new(service))
            .serve_with_incoming(stream)
            .await?;

        Ok(())
    }
}

struct ProviderService<P: ProviderV2> {
    provider: Arc<RwLock<P>>,
}

#[tonic::async_trait]
impl<P: ProviderV2 + 'static> ProtoProvider for ProviderService<P> {
    async fn get_provider_schema(
        &self,
        _request: Request<get_provider_schema::Request>,
    ) -> std::result::Result<Response<get_provider_schema::Response>, Status> {
        // Get cached schemas from provider
        let provider = self.provider.read().await;
        let data_source_schemas = provider.data_source_schemas().await;
        let resource_schemas = provider.resource_schemas().await;

        // Build provider schema
        let provider_schema = Schema {
            version: 0,
            block: Some(schema::Block {
                version: 0,
                attributes: vec![
                    schema::Attribute {
                        name: "endpoint".to_string(),
                        r#type: attribute_type_to_bytes(&AttributeType::String),
                        description: "Proxmox API endpoint URL (can also be set via PROXMOX_ENDPOINT env var)".to_string(),
                        required: false,
                        optional: true,
                        computed: false,
                        sensitive: false,
                        description_kind: StringKind::Plain as i32,
                        deprecated: false,
                        nested_type: None,
                        write_only: false,
                    },
                    schema::Attribute {
                        name: "api_token".to_string(),
                        r#type: attribute_type_to_bytes(&AttributeType::String),
                        description: "API token for authentication (can also be set via PROXMOX_API_TOKEN env var)".to_string(),
                        required: false,
                        optional: true,
                        computed: false,
                        sensitive: true,
                        description_kind: StringKind::Plain as i32,
                        deprecated: false,
                        nested_type: None,
                        write_only: false,
                    },
                    schema::Attribute {
                        name: "insecure".to_string(),
                        r#type: attribute_type_to_bytes(&AttributeType::Bool),
                        description: "Skip TLS certificate verification".to_string(),
                        required: false,
                        optional: true,
                        computed: false,
                        sensitive: false,
                        description_kind: StringKind::Plain as i32,
                        deprecated: false,
                        nested_type: None,
                        write_only: false,
                    },
                ],
                block_types: vec![],
                description: "Proxmox provider configuration".to_string(),
                description_kind: StringKind::Plain as i32,
                deprecated: false,
            }),
        };

        // Convert data source schemas
        let mut data_sources = HashMap::new();
        for (name, schema) in data_source_schemas {
            data_sources.insert(
                name.clone(),
                Schema {
                    version: schema.version,
                    block: Some(schema::Block {
                        version: schema.version,
                        attributes: schema
                            .attributes
                            .values()
                            .map(|attr| schema::Attribute {
                                name: attr.name.clone(),
                                r#type: attribute_type_to_bytes(&attr.r#type),
                                description: attr.description.clone(),
                                required: attr.required,
                                optional: attr.optional,
                                computed: attr.computed,
                                sensitive: attr.sensitive,
                                description_kind: StringKind::Plain as i32,
                                deprecated: false,
                                nested_type: None,
                                write_only: false,
                            })
                            .collect(),
                        block_types: vec![],
                        description: String::new(),
                        description_kind: StringKind::Plain as i32,
                        deprecated: false,
                    }),
                },
            );
        }

        // Convert resource schemas
        let mut resources = HashMap::new();
        for (name, schema) in resource_schemas {
            resources.insert(
                name.clone(),
                Schema {
                    version: schema.version,
                    block: Some(schema::Block {
                        version: schema.version,
                        attributes: schema
                            .attributes
                            .values()
                            .map(|attr| schema::Attribute {
                                name: attr.name.clone(),
                                r#type: attribute_type_to_bytes(&attr.r#type),
                                description: attr.description.clone(),
                                required: attr.required,
                                optional: attr.optional,
                                computed: attr.computed,
                                sensitive: attr.sensitive,
                                description_kind: StringKind::Plain as i32,
                                deprecated: false,
                                nested_type: None,
                                write_only: false,
                            })
                            .collect(),
                        block_types: vec![],
                        description: String::new(),
                        description_kind: StringKind::Plain as i32,
                        deprecated: false,
                    }),
                },
            );
        }

        Ok(Response::new(get_provider_schema::Response {
            provider: Some(provider_schema),
            resource_schemas: resources,
            data_source_schemas: data_sources,
            functions: HashMap::new(),
            ephemeral_resource_schemas: HashMap::new(),
            diagnostics: vec![],
            provider_meta: None,
            server_capabilities: Some(ServerCapabilities {
                plan_destroy: false,
                get_provider_schema_optional: false,
                move_resource_state: false,
            }),
        }))
    }

    async fn validate_provider_config(
        &self,
        request: Request<validate_provider_config::Request>,
    ) -> std::result::Result<Response<validate_provider_config::Response>, Status> {
        let req = request.into_inner();
        let _config = decode_dynamic_value(&req.config)?;

        Ok(Response::new(validate_provider_config::Response {
            diagnostics: vec![],
        }))
    }

    async fn configure_provider(
        &self,
        request: Request<configure_provider::Request>,
    ) -> std::result::Result<Response<configure_provider::Response>, Status> {
        let req = request.into_inner();
        let config = decode_dynamic_value(&req.config)?;

        eprintln!(
            "DEBUG: configure_provider called with config: {:?}",
            config.values.keys().collect::<Vec<_>>()
        );

        // Configure the provider with the provided configuration
        let configure_req = crate::request::ConfigureRequest {
            context: Context::new(),
            config: Config {
                values: config.values,
            },
        };

        // Actually configure the provider
        let mut provider = self.provider.write().await;
        let response = provider.configure(configure_req).await;

        eprintln!("DEBUG: Provider configured successfully");

        // Convert diagnostics
        let diagnostics = convert_diagnostics(response.diagnostics);

        Ok(Response::new(configure_provider::Response { diagnostics }))
    }

    async fn stop_provider(
        &self,
        _request: Request<stop_provider::Request>,
    ) -> std::result::Result<Response<stop_provider::Response>, Status> {
        Ok(Response::new(stop_provider::Response {
            error: String::new(),
        }))
    }

    async fn validate_resource_config(
        &self,
        request: Request<validate_resource_config::Request>,
    ) -> std::result::Result<Response<validate_resource_config::Response>, Status> {
        let req = request.into_inner();
        let type_name = req.type_name;

        // Get the resource schema from cached schemas
        let provider = self.provider.read().await;
        let schemas = provider.resource_schemas().await;
        let schema = match schemas.get(&type_name) {
            Some(s) => s,
            None => {
                return Ok(Response::new(validate_resource_config::Response {
                    diagnostics: vec![Diagnostic {
                        severity: diagnostic::Severity::Error as i32,
                        summary: format!("Unknown resource type: {}", type_name),
                        detail: String::new(),
                        attribute: None,
                    }],
                }))
            }
        };

        // Try to decode the configuration
        let config = match decode_dynamic_value(&req.config) {
            Ok(config) => config,
            Err(e) => {
                // If decoding fails due to unknown values, we can't fully validate
                // but this is acceptable during planning phase
                if e.to_string().contains("data did not match any variant") {
                    eprintln!("DEBUG: Skipping validation due to unknown values in config");
                    return Ok(Response::new(validate_resource_config::Response {
                        diagnostics: vec![],
                    }));
                } else {
                    return Err(e);
                }
            }
        };

        let mut diagnostics = Vec::new();

        // Validate required fields
        for (attr_name, attr) in &schema.attributes {
            if attr.required && !config.values.contains_key::<str>(attr_name) {
                diagnostics.push(Diagnostic {
                    severity: diagnostic::Severity::Error as i32,
                    summary: format!("Missing required field: {}", attr_name),
                    detail: format!("The field '{}' is required but was not provided", attr_name),
                    attribute: Some(AttributePath {
                        steps: vec![attribute_path::Step {
                            selector: Some(attribute_path::step::Selector::AttributeName(
                                attr_name.to_string(),
                            )),
                        }],
                    }),
                });
            }
        }

        // Validate field types and check for unknown fields
        for (field_name, value) in &config.values {
            match schema.attributes.get(field_name) {
                Some(attr) => {
                    // Validate type matches
                    if !validate_dynamic_type(value, &attr.r#type) {
                        diagnostics.push(Diagnostic {
                            severity: diagnostic::Severity::Error as i32,
                            summary: format!("Type mismatch for field: {}", field_name),
                            detail: format!(
                                "Field '{}' expects type {:?} but got {:?}",
                                field_name,
                                attr.r#type,
                                dynamic_type_name(value)
                            ),
                            attribute: Some(AttributePath {
                                steps: vec![attribute_path::Step {
                                    selector: Some(attribute_path::step::Selector::AttributeName(
                                        field_name.clone(),
                                    )),
                                }],
                            }),
                        });
                    }
                }
                None => {
                    // Unknown field
                    diagnostics.push(Diagnostic {
                        severity: diagnostic::Severity::Error as i32,
                        summary: format!("Unknown field: {}", field_name),
                        detail: format!(
                            "The field '{}' is not defined in the resource schema",
                            field_name
                        ),
                        attribute: Some(AttributePath {
                            steps: vec![attribute_path::Step {
                                selector: Some(attribute_path::step::Selector::AttributeName(
                                    field_name.clone(),
                                )),
                            }],
                        }),
                    });
                }
            }
        }

        Ok(Response::new(validate_resource_config::Response {
            diagnostics,
        }))
    }

    async fn validate_data_resource_config(
        &self,
        _request: Request<validate_data_resource_config::Request>,
    ) -> std::result::Result<Response<validate_data_resource_config::Response>, Status> {
        Ok(Response::new(validate_data_resource_config::Response {
            diagnostics: vec![],
        }))
    }

    async fn read_resource(
        &self,
        request: Request<read_resource::Request>,
    ) -> std::result::Result<Response<read_resource::Response>, Status> {
        let req = request.into_inner();
        let type_name = req.type_name;

        // Create a new resource instance using the factory
        let provider = self.provider.read().await;
        let resource = provider
            .create_resource(&type_name)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let current_state = decode_dynamic_value(&req.current_state)?;
        let state = State {
            values: current_state.values,
        };

        let context = Context::new();
        let read_req = ReadRequest {
            context,
            current_state: state,
        };

        let read_resp = resource.read(read_req).await;

        let (new_state_value, encoded_state) = match read_resp.state {
            Some(state) => {
                let encoded = encode_state(&state)?;
                (encoded, vec![])
            }
            None => match &req.current_state {
                Some(state) => (state.clone(), vec![]),
                None => (
                    DynamicValue {
                        msgpack: vec![],
                        json: vec![],
                    },
                    vec![],
                ),
            },
        };

        Ok(Response::new(read_resource::Response {
            new_state: Some(new_state_value),
            diagnostics: convert_diagnostics(read_resp.diagnostics),
            private: encoded_state,
            deferred: None,
            new_identity: None,
        }))
    }

    async fn plan_resource_change(
        &self,
        request: Request<plan_resource_change::Request>,
    ) -> std::result::Result<Response<plan_resource_change::Response>, Status> {
        use crate::plan_modifier::PlanModifyRequest;
        use crate::proto::tfplugin6::attribute_path::Step;
        use crate::proto::tfplugin6::AttributePath;
        use crate::types::Dynamic;

        let req = request.into_inner();
        let type_name = req.type_name.clone();

        // Get the resource schema to access plan modifiers
        let provider = self.provider.read().await;
        let resource_schemas = provider.resource_schemas().await;
        let resource_schema = resource_schemas
            .get(&type_name)
            .ok_or_else(|| Status::not_found(format!("Unknown resource type: {}", type_name)))?;

        let prior_state = decode_dynamic_value(&req.prior_state)?.values;
        let config = decode_dynamic_value(&req.config)?.values;
        let proposed_new_state = decode_dynamic_value(&req.proposed_new_state)?.values;

        // For planning, we mostly just return the proposed state
        // Real validation happens during apply
        let mut planned_state = if prior_state.is_empty() && !proposed_new_state.is_empty() {
            // This is a create operation
            proposed_new_state
        } else if !prior_state.is_empty() && proposed_new_state.is_empty() {
            // This is a delete operation
            HashMap::new()
        } else {
            // This is an update operation
            proposed_new_state
        };

        let mut requires_replace = Vec::new();
        let mut all_diagnostics = TfplugDiagnostics::new();

        // Apply defaults for attributes with null config values
        for (attr_name, attr_schema) in &resource_schema.attributes {
            if let Some(default) = &attr_schema.default {
                // Only apply default if config value is null and attribute is optional+computed
                if attr_schema.optional && attr_schema.computed {
                    let config_value = config
                        .get::<str>(attr_name)
                        .cloned()
                        .unwrap_or(Dynamic::Null);
                    if matches!(config_value, Dynamic::Null) {
                        let default_request = crate::defaults::DefaultRequest {
                            attribute_path: attr_name.to_string(),
                        };
                        let default_response = default.default_value(default_request);
                        planned_state.insert(attr_name.to_string(), default_response.value);
                    }
                }
            }
        }

        // Apply plan modifiers for each attribute
        for (attr_name, attr_schema) in &resource_schema.attributes {
            if !attr_schema.plan_modifiers.is_empty() {
                // Get the values for this attribute
                let state_value = prior_state
                    .get::<str>(attr_name)
                    .cloned()
                    .unwrap_or(Dynamic::Null);
                let plan_value = planned_state
                    .get(attr_name)
                    .cloned()
                    .unwrap_or(Dynamic::Null);
                let config_value = config.get(attr_name).cloned().unwrap_or(Dynamic::Null);

                let mut current_plan_value = plan_value.clone();

                // Apply each plan modifier
                for modifier in &attr_schema.plan_modifiers {
                    let request = PlanModifyRequest {
                        state: state_value.clone(),
                        plan: current_plan_value.clone(),
                        config: config_value.clone(),
                        attribute_path: attr_name.clone(),
                    };

                    let response = modifier.modify_plan(request);

                    // Update the plan value if it was modified
                    current_plan_value = response.plan_value;

                    // Check if this attribute requires replacement
                    if response.requires_replace {
                        requires_replace.push(AttributePath {
                            steps: vec![Step {
                                selector: Some(
                                    crate::proto::tfplugin6::attribute_path::step::Selector::AttributeName(
                                        attr_name.to_string(),
                                    ),
                                ),
                            }],
                        });
                    }

                    // Collect diagnostics
                    for error in &response.diagnostics.errors {
                        all_diagnostics.add_error(&error.summary, error.detail.as_deref());
                    }
                    for warning in &response.diagnostics.warnings {
                        all_diagnostics.add_warning(&warning.summary, warning.detail.as_deref());
                    }
                }

                // Update the planned state with the potentially modified value
                match current_plan_value {
                    Dynamic::Null => {
                        planned_state.remove(attr_name);
                    }
                    _ => {
                        planned_state.insert(attr_name.to_string(), current_plan_value);
                    }
                }
            }
        }

        let encoded_planned_state = encode_dynamic_values(&planned_state)?;

        Ok(Response::new(plan_resource_change::Response {
            planned_state: Some(encoded_planned_state),
            requires_replace,
            planned_private: vec![],
            diagnostics: encode_diagnostics(&all_diagnostics),
            legacy_type_system: false,
            deferred: None,
            planned_identity: None,
        }))
    }

    async fn apply_resource_change(
        &self,
        request: Request<apply_resource_change::Request>,
    ) -> std::result::Result<Response<apply_resource_change::Response>, Status> {
        let req = request.into_inner();
        let type_name = req.type_name;

        // Create a new resource instance using the factory
        let provider = self.provider.read().await;
        let resource = provider
            .create_resource(&type_name)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let prior_state = decode_dynamic_value(&req.prior_state)?.values;
        let config = decode_dynamic_value(&req.config)?.values;
        let planned_state = decode_dynamic_value(&req.planned_state)?.values;

        let context = Context::new();

        // Determine the operation type
        let is_create = prior_state.is_empty() && !planned_state.is_empty();
        let is_delete = !prior_state.is_empty() && planned_state.is_empty();
        let is_update = !prior_state.is_empty() && !planned_state.is_empty();

        let (new_state, diagnostics) = if is_create {
            // Create operation
            let create_req = CreateRequest {
                context,
                config: Config { values: config },
                planned_state: State {
                    values: planned_state.clone(),
                },
            };
            let create_resp = resource.create(create_req).await;
            (create_resp.state, create_resp.diagnostics)
        } else if is_delete {
            // Delete operation
            let delete_req = DeleteRequest {
                context,
                current_state: State {
                    values: prior_state.clone(),
                },
            };
            let delete_resp = resource.delete(delete_req).await;
            (
                State {
                    values: HashMap::new(),
                },
                delete_resp.diagnostics,
            )
        } else if is_update {
            // Update operation
            let update_req = UpdateRequest {
                context,
                config: Config {
                    values: config.clone(),
                },
                planned_state: State {
                    values: planned_state.clone(),
                },
                current_state: State {
                    values: prior_state.clone(),
                },
            };
            let update_resp = resource.update(update_req).await;
            (update_resp.state, update_resp.diagnostics)
        } else {
            // No-op
            (
                State {
                    values: planned_state.clone(),
                },
                TfplugDiagnostics::new(),
            )
        };

        // Check if there were any errors
        if !diagnostics.errors.is_empty() {
            // For create operations that fail, we should return the planned state
            // so Terraform can retry. For other operations, return the prior state.
            let state_to_return = if is_create {
                &planned_state
            } else {
                &prior_state
            };

            Ok(Response::new(apply_resource_change::Response {
                new_state: Some(encode_dynamic_values(state_to_return)?),
                diagnostics: encode_diagnostics(&diagnostics),
                private: vec![],
                legacy_type_system: false,
                new_identity: None,
            }))
        } else {
            // Success - return the new state
            let new_state_value = if is_delete && new_state.values.is_empty() {
                None
            } else {
                Some(encode_state(&new_state)?)
            };

            Ok(Response::new(apply_resource_change::Response {
                new_state: new_state_value,
                diagnostics: encode_diagnostics(&diagnostics),
                private: vec![],
                legacy_type_system: false,
                new_identity: None,
            }))
        }
    }

    async fn import_resource_state(
        &self,
        _request: Request<import_resource_state::Request>,
    ) -> std::result::Result<Response<import_resource_state::Response>, Status> {
        todo!()
    }

    async fn move_resource_state(
        &self,
        _request: Request<move_resource_state::Request>,
    ) -> std::result::Result<Response<move_resource_state::Response>, Status> {
        todo!()
    }

    async fn read_data_source(
        &self,
        request: Request<read_data_source::Request>,
    ) -> std::result::Result<Response<read_data_source::Response>, Status> {
        let req = request.into_inner();
        let type_name = req.type_name;
        let config = decode_dynamic_value(&req.config)?;

        eprintln!("DEBUG: read_data_source called for {}", type_name);

        // Create a new data source instance using the factory
        let provider = self.provider.read().await;
        let data_source = provider
            .create_data_source(&type_name)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        eprintln!("DEBUG: Calling read on data source");

        let context = Context::new();
        let read_req = ReadRequest {
            context,
            current_state: State {
                values: config.values,
            },
        };

        let read_resp = data_source.read(read_req).await;

        eprintln!("DEBUG: Read completed successfully");

        let state_value = match read_resp.state {
            Some(state) => Some(encode_dynamic_value(&state)?),
            None => None,
        };

        Ok(Response::new(read_data_source::Response {
            state: state_value,
            diagnostics: convert_diagnostics(read_resp.diagnostics),
            deferred: None,
        }))
    }

    async fn get_functions(
        &self,
        _request: Request<get_functions::Request>,
    ) -> std::result::Result<Response<get_functions::Response>, Status> {
        Ok(Response::new(get_functions::Response {
            functions: HashMap::new(),
            diagnostics: vec![],
        }))
    }

    async fn call_function(
        &self,
        _request: Request<call_function::Request>,
    ) -> std::result::Result<Response<call_function::Response>, Status> {
        todo!()
    }

    async fn get_metadata(
        &self,
        _request: Request<get_metadata::Request>,
    ) -> std::result::Result<Response<get_metadata::Response>, Status> {
        Ok(Response::new(get_metadata::Response {
            server_capabilities: Some(ServerCapabilities {
                plan_destroy: false,
                get_provider_schema_optional: false,
                move_resource_state: false,
            }),
            diagnostics: vec![],
            data_sources: vec![],
            resources: vec![],
            functions: vec![],
            ephemeral_resources: vec![],
        }))
    }

    async fn get_resource_identity_schemas(
        &self,
        _request: Request<get_resource_identity_schemas::Request>,
    ) -> std::result::Result<Response<get_resource_identity_schemas::Response>, Status> {
        Ok(Response::new(get_resource_identity_schemas::Response {
            identity_schemas: HashMap::new(),
            diagnostics: vec![],
        }))
    }

    async fn upgrade_resource_state(
        &self,
        request: Request<upgrade_resource_state::Request>,
    ) -> std::result::Result<Response<upgrade_resource_state::Response>, Status> {
        let req = request.into_inner();

        // For now, we don't handle state upgrades - just return the raw state as-is
        // Convert RawState to DynamicValue
        let upgraded_state = req.raw_state.as_ref().map(|raw| DynamicValue {
            msgpack: vec![], // We'll use JSON for now
            json: raw.json.clone(),
        });

        Ok(Response::new(upgrade_resource_state::Response {
            upgraded_state,
            diagnostics: vec![],
        }))
    }

    async fn upgrade_resource_identity(
        &self,
        _request: Request<upgrade_resource_identity::Request>,
    ) -> std::result::Result<Response<upgrade_resource_identity::Response>, Status> {
        todo!()
    }

    async fn validate_ephemeral_resource_config(
        &self,
        _request: Request<validate_ephemeral_resource_config::Request>,
    ) -> std::result::Result<Response<validate_ephemeral_resource_config::Response>, Status> {
        Ok(Response::new(
            validate_ephemeral_resource_config::Response {
                diagnostics: vec![],
            },
        ))
    }

    async fn open_ephemeral_resource(
        &self,
        _request: Request<open_ephemeral_resource::Request>,
    ) -> std::result::Result<Response<open_ephemeral_resource::Response>, Status> {
        todo!()
    }

    async fn renew_ephemeral_resource(
        &self,
        _request: Request<renew_ephemeral_resource::Request>,
    ) -> std::result::Result<Response<renew_ephemeral_resource::Response>, Status> {
        todo!()
    }

    async fn close_ephemeral_resource(
        &self,
        _request: Request<close_ephemeral_resource::Request>,
    ) -> std::result::Result<Response<close_ephemeral_resource::Response>, Status> {
        todo!()
    }
}

// Helper functions

fn attribute_type_to_bytes(attr_type: &AttributeType) -> Vec<u8> {
    match attr_type {
        AttributeType::String => "\"string\"".as_bytes().to_vec(),
        AttributeType::Number => "\"number\"".as_bytes().to_vec(),
        AttributeType::Bool => "\"bool\"".as_bytes().to_vec(),
        AttributeType::List(elem) => {
            let elem_type = attribute_type_to_bytes(elem);
            format!("[\"list\", {}]", String::from_utf8_lossy(&elem_type)).into_bytes()
        }
        AttributeType::Set(elem) => {
            let elem_type = attribute_type_to_bytes(elem);
            format!("[\"set\", {}]", String::from_utf8_lossy(&elem_type)).into_bytes()
        }
        AttributeType::Map(elem) => {
            let elem_type = attribute_type_to_bytes(elem);
            format!("[\"map\", {}]", String::from_utf8_lossy(&elem_type)).into_bytes()
        }
        AttributeType::Object(attrs) => {
            let attrs_json: Vec<String> = attrs
                .iter()
                .map(|(name, attr_type)| {
                    format!(
                        "\"{}\": {}",
                        name,
                        String::from_utf8_lossy(&attribute_type_to_bytes(attr_type))
                    )
                })
                .collect();
            format!("[\"object\", {{{}}}]", attrs_json.join(", ")).into_bytes()
        }
    }
}

#[allow(clippy::result_large_err)]
fn decode_dynamic_value(value: &Option<DynamicValue>) -> std::result::Result<Config, Status> {
    let value = match value {
        Some(v) => v,
        None => {
            return Ok(Config {
                values: HashMap::new(),
            })
        }
    };

    if !value.msgpack.is_empty() {
        // Try to decode as a map first, if that fails and it's null/unit, return empty
        match decode::from_slice::<HashMap<String, Dynamic>>(&value.msgpack) {
            Ok(values) => Ok(Config { values }),
            Err(e) => {
                // Check if it's just a null/unit value
                match decode::from_slice::<Option<HashMap<String, Dynamic>>>(&value.msgpack) {
                    Ok(None) => Ok(Config {
                        values: HashMap::new(),
                    }),
                    Ok(Some(values)) => Ok(Config { values }),
                    Err(_) => {
                        // Log first few bytes to understand what format we're getting
                        let preview = &value.msgpack[..value.msgpack.len().min(50)];
                        eprintln!("DEBUG: Unknown msgpack format. First bytes: {:?}", preview);
                        Err(Status::invalid_argument(format!(
                            "Failed to decode msgpack: {}",
                            e
                        )))
                    }
                }
            }
        }
    } else if !value.json.is_empty() {
        let values: HashMap<String, Dynamic> = serde_json::from_slice(&value.json)
            .map_err(|e| Status::invalid_argument(format!("Failed to decode json: {}", e)))?;
        Ok(Config { values })
    } else {
        Ok(Config {
            values: HashMap::new(),
        })
    }
}

#[allow(clippy::result_large_err)]
fn encode_dynamic_value(state: &State) -> std::result::Result<DynamicValue, Status> {
    let msgpack = encode::to_vec_named(&state.values)
        .map_err(|e| Status::internal(format!("Failed to encode msgpack: {}", e)))?;

    Ok(DynamicValue {
        msgpack,
        json: vec![],
    })
}

#[allow(clippy::result_large_err)]
fn encode_state(state: &State) -> std::result::Result<DynamicValue, Status> {
    encode_dynamic_value(state)
}

#[allow(clippy::result_large_err)]
fn encode_dynamic_values(
    values: &HashMap<String, Dynamic>,
) -> std::result::Result<DynamicValue, Status> {
    let state = State {
        values: values.clone(),
    };
    encode_dynamic_value(&state)
}

fn validate_dynamic_type(value: &Dynamic, expected_type: &AttributeType) -> bool {
    match (value, expected_type) {
        (Dynamic::Null, _) => true, // Null values are acceptable (might be computed/unknown)
        (Dynamic::String(_), AttributeType::String) => true,
        (Dynamic::Number(_), AttributeType::Number) => true,
        (Dynamic::Bool(_), AttributeType::Bool) => true,
        (Dynamic::List(list), AttributeType::List(elem_type)) => list
            .iter()
            .all(|elem| validate_dynamic_type(elem, elem_type)),
        (Dynamic::List(list), AttributeType::Set(elem_type)) => list
            .iter()
            .all(|elem| validate_dynamic_type(elem, elem_type)),
        (Dynamic::Map(map), AttributeType::Map(elem_type)) => map
            .values()
            .all(|elem| validate_dynamic_type(elem, elem_type)),
        (Dynamic::Map(map), AttributeType::Object(attrs)) => {
            // For objects, validate each field has the correct type
            for (field_name, field_type) in attrs {
                if let Some(value) = map.get(field_name) {
                    if !validate_dynamic_type(value, field_type) {
                        return false;
                    }
                }
            }
            true
        }
        _ => false,
    }
}

fn dynamic_type_name(value: &Dynamic) -> &'static str {
    match value {
        Dynamic::Null => "null",
        Dynamic::Bool(_) => "bool",
        Dynamic::Number(_) => "number",
        Dynamic::String(_) => "string",
        Dynamic::List(_) => "list",
        Dynamic::Map(_) => "map",
        Dynamic::Unknown => "unknown",
    }
}

fn encode_diagnostics(diags: &TfplugDiagnostics) -> Vec<Diagnostic> {
    diags
        .errors
        .iter()
        .map(|e| Diagnostic {
            severity: diagnostic::Severity::Error as i32,
            summary: e.summary.clone(),
            detail: e.detail.clone().unwrap_or_default(),
            attribute: None,
        })
        .chain(diags.warnings.iter().map(|w| Diagnostic {
            severity: diagnostic::Severity::Warning as i32,
            summary: w.summary.clone(),
            detail: w.detail.clone().unwrap_or_default(),
            attribute: None,
        }))
        .collect()
}

fn convert_diagnostics(diags: TfplugDiagnostics) -> Vec<Diagnostic> {
    let mut result = Vec::new();

    for diag in diags.errors {
        result.push(Diagnostic {
            severity: diagnostic::Severity::Error as i32,
            summary: diag.summary,
            detail: diag.detail.unwrap_or_default(),
            attribute: None,
        });
    }

    for diag in diags.warnings {
        result.push(Diagnostic {
            severity: diagnostic::Severity::Warning as i32,
            summary: diag.summary,
            detail: diag.detail.unwrap_or_default(),
            attribute: None,
        });
    }

    result
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::provider::{DataSourceSchema, ResourceSchema};
    use crate::provider::{DataSourceV2, ResourceV2};
    use crate::request::{
        ConfigureRequest, ConfigureResponse, CreateRequest, CreateResponse,
        DataSourceSchemaResponse, DeleteRequest, DeleteResponse, ReadRequest, ReadResponse,
        ResourceSchemaResponse, SchemaRequest, UpdateRequest, UpdateResponse,
    };
    use crate::types::{Dynamic, State};
    use crate::AttributeBuilder;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct TestProvider {
        configured: Arc<Mutex<bool>>,
        resource_schemas: HashMap<String, ResourceSchema>,
        data_source_schemas: HashMap<String, DataSourceSchema>,
    }

    impl TestProvider {
        fn new() -> Self {
            let mut resource_schemas = HashMap::new();
            let mut resource_attributes = HashMap::new();
            let id_attr = AttributeBuilder::string("id")
                .description("Resource ID")
                .computed()
                .build();
            resource_attributes.insert("id".to_string(), id_attr);
            resource_schemas.insert(
                "test_resource".to_string(),
                ResourceSchema {
                    version: 0,
                    attributes: resource_attributes,
                },
            );

            let mut data_source_schemas = HashMap::new();
            let mut data_source_attributes = HashMap::new();
            let value_attr = AttributeBuilder::string("value")
                .description("Test value")
                .computed()
                .build();
            data_source_attributes.insert("value".to_string(), value_attr);
            data_source_schemas.insert(
                "test_data".to_string(),
                DataSourceSchema {
                    version: 0,
                    attributes: data_source_attributes,
                },
            );

            Self {
                configured: Arc::new(Mutex::new(false)),
                resource_schemas,
                data_source_schemas,
            }
        }
    }

    #[async_trait]
    impl ProviderV2 for TestProvider {
        async fn configure(&mut self, _request: ConfigureRequest) -> ConfigureResponse {
            let mut configured = self.configured.lock().await;
            *configured = true;
            ConfigureResponse {
                diagnostics: TfplugDiagnostics::new(),
            }
        }

        async fn create_resource(&self, name: &str) -> Result<Box<dyn ResourceV2>> {
            match name {
                "test_resource" => Ok(Box::new(TestResource::new())),
                _ => Err(format!("Unknown resource type: {}", name).into()),
            }
        }

        async fn create_data_source(&self, name: &str) -> Result<Box<dyn DataSourceV2>> {
            match name {
                "test_data" => Ok(Box::new(TestDataSource::new())),
                _ => Err(format!("Unknown data source type: {}", name).into()),
            }
        }

        async fn resource_schemas(&self) -> HashMap<String, ResourceSchema> {
            self.resource_schemas.clone()
        }

        async fn data_source_schemas(&self) -> HashMap<String, DataSourceSchema> {
            self.data_source_schemas.clone()
        }
    }

    struct TestResource;

    impl TestResource {
        fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl ResourceV2 for TestResource {
        async fn schema(&self, _request: SchemaRequest) -> ResourceSchemaResponse {
            ResourceSchemaResponse {
                schema: ResourceSchema {
                    version: 0,
                    attributes: HashMap::new(),
                },
                diagnostics: TfplugDiagnostics::new(),
            }
        }

        async fn create(&self, _request: CreateRequest) -> CreateResponse {
            let mut state = State::new();
            state
                .values
                .insert("id".to_string(), Dynamic::String("test-123".to_string()));
            CreateResponse {
                state,
                diagnostics: TfplugDiagnostics::new(),
            }
        }

        async fn read(&self, request: ReadRequest) -> ReadResponse {
            ReadResponse {
                state: Some(request.current_state),
                diagnostics: TfplugDiagnostics::new(),
            }
        }

        async fn update(&self, request: UpdateRequest) -> UpdateResponse {
            UpdateResponse {
                state: request.planned_state,
                diagnostics: TfplugDiagnostics::new(),
            }
        }

        async fn delete(&self, _request: DeleteRequest) -> DeleteResponse {
            DeleteResponse {
                diagnostics: TfplugDiagnostics::new(),
            }
        }
    }

    struct TestDataSource;

    impl TestDataSource {
        fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl DataSourceV2 for TestDataSource {
        async fn schema(&self, _request: SchemaRequest) -> DataSourceSchemaResponse {
            DataSourceSchemaResponse {
                schema: DataSourceSchema {
                    version: 0,
                    attributes: HashMap::new(),
                },
                diagnostics: TfplugDiagnostics::new(),
            }
        }

        async fn read(&self, _request: ReadRequest) -> ReadResponse {
            let mut state = State::new();
            state.values.insert(
                "value".to_string(),
                Dynamic::String("test-value".to_string()),
            );
            ReadResponse {
                state: Some(state),
                diagnostics: TfplugDiagnostics::new(),
            }
        }
    }

    #[tokio::test]
    async fn grpc_service_calls_async_provider_methods() {
        let provider = TestProvider::new();
        let service = ProviderService {
            provider: Arc::new(RwLock::new(provider)),
        };

        // Test get_provider_schema
        let schema_req = Request::new(get_provider_schema::Request {});
        let schema_resp = service.get_provider_schema(schema_req).await.unwrap();
        let inner = schema_resp.into_inner();
        assert!(inner.resource_schemas.contains_key("test_resource"));
        assert!(inner.data_source_schemas.contains_key("test_data"));

        // Test read_data_source with async factory
        let read_req = Request::new(read_data_source::Request {
            type_name: "test_data".to_string(),
            config: Some(DynamicValue {
                msgpack: encode::to_vec_named(&HashMap::<String, Dynamic>::new()).unwrap(),
                json: vec![],
            }),
            provider_meta: None,
            client_capabilities: None,
        });
        let read_resp = service.read_data_source(read_req).await.unwrap();
        assert!(read_resp.into_inner().state.is_some());
    }

    #[tokio::test]
    async fn grpc_service_handles_async_resource_operations() {
        let provider = TestProvider::new();
        let service = ProviderService {
            provider: Arc::new(RwLock::new(provider)),
        };

        // Test apply_resource_change with create operation
        let mut planned_state = HashMap::new();
        planned_state.insert("id".to_string(), Dynamic::String("test-123".to_string()));

        let apply_req = Request::new(apply_resource_change::Request {
            type_name: "test_resource".to_string(),
            prior_state: Some(DynamicValue {
                msgpack: encode::to_vec_named(&HashMap::<String, Dynamic>::new()).unwrap(),
                json: vec![],
            }),
            planned_state: Some(DynamicValue {
                msgpack: encode::to_vec_named(&planned_state).unwrap(),
                json: vec![],
            }),
            config: Some(DynamicValue {
                msgpack: encode::to_vec_named(&HashMap::<String, Dynamic>::new()).unwrap(),
                json: vec![],
            }),
            planned_private: vec![],
            provider_meta: None,
            planned_identity: None,
        });

        let apply_resp = service.apply_resource_change(apply_req).await.unwrap();
        assert!(apply_resp.into_inner().new_state.is_some());
    }

    #[tokio::test]
    async fn grpc_service_propagates_async_errors() {
        let provider = TestProvider::new();
        let service = ProviderService {
            provider: Arc::new(RwLock::new(provider)),
        };

        // Test with non-existent resource
        let apply_req = Request::new(apply_resource_change::Request {
            type_name: "non_existent".to_string(),
            prior_state: Some(DynamicValue {
                msgpack: encode::to_vec_named(&HashMap::<String, Dynamic>::new()).unwrap(),
                json: vec![],
            }),
            planned_state: Some(DynamicValue {
                msgpack: encode::to_vec_named(&HashMap::<String, Dynamic>::new()).unwrap(),
                json: vec![],
            }),
            config: Some(DynamicValue {
                msgpack: encode::to_vec_named(&HashMap::<String, Dynamic>::new()).unwrap(),
                json: vec![],
            }),
            planned_private: vec![],
            provider_meta: None,
            planned_identity: None,
        });

        let result = service.apply_resource_change(apply_req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("non_existent"));
    }
}
