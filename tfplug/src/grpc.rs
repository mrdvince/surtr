use crate::proto::tfplugin6::{
    provider_server::{Provider as ProtoProvider, ProviderServer as ProtoProviderServer},
    *,
};
use crate::provider::Provider;
use crate::types::{Config, Diagnostics as TfplugDiagnostics, Dynamic, State};
use crate::Result;
use rmp_serde::{decode, encode};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status};

pub struct ProviderServer<P> {
    provider: Arc<Mutex<P>>,
    cert_path: PathBuf,
    key_path: PathBuf,
}

impl<P: Provider + 'static> ProviderServer<P> {
    pub fn new(provider: P, cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            provider: Arc::new(Mutex::new(provider)),
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

struct ProviderService<P> {
    provider: Arc<Mutex<P>>,
}

#[tonic::async_trait]
impl<P: Provider + 'static> ProtoProvider for ProviderService<P> {
    async fn get_provider_schema(
        &self,
        _request: Request<get_provider_schema::Request>,
    ) -> std::result::Result<Response<get_provider_schema::Response>, Status> {
        let provider = self.provider.lock().unwrap();
        let data_source_schemas = provider.get_schema();
        let resource_schemas = provider.get_resource_schemas();

        let provider_schema = Schema {
            version: 0,
            block: Some(schema::Block {
                version: 0,
                attributes: vec![
                    schema::Attribute {
                        name: "endpoint".to_string(),
                        r#type: string_type(),
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
                        r#type: string_type(),
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
                        r#type: bool_type(),
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

        let mut data_sources = HashMap::new();
        for (name, schema) in data_source_schemas {
            data_sources.insert(
                name,
                Schema {
                    version: schema.version,
                    block: Some(schema::Block {
                        version: schema.version,
                        attributes: schema
                            .attributes
                            .into_values()
                            .map(|attr| schema::Attribute {
                                name: attr.name.clone(),
                                r#type: attr.r#type.clone(),
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

        let mut resources = HashMap::new();
        for (name, schema) in resource_schemas {
            resources.insert(
                name,
                Schema {
                    version: schema.version,
                    block: Some(schema::Block {
                        version: schema.version,
                        attributes: schema
                            .attributes
                            .into_values()
                            .map(|attr| schema::Attribute {
                                name: attr.name.clone(),
                                r#type: attr.r#type.clone(),
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

        let mut provider = self.provider.lock().unwrap();
        let diags = provider
            .configure(config)
            .map_err(|e| Status::internal(format!("Failed to configure provider: {}", e)))?;

        eprintln!("DEBUG: Provider configured successfully");

        Ok(Response::new(configure_provider::Response {
            diagnostics: convert_diagnostics(diags),
        }))
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
        let _type_name = req.type_name;

        // Try to decode, but if it fails due to unknowns, just skip validation
        match decode_dynamic_value(&req.config) {
            Ok(_config) => {
                // For now, we'll just return success
                // Real validation would check required fields, types, etc.
            }
            Err(e) => {
                // If decoding fails, check if it's due to unknown values
                // In that case, we can't validate yet, so just return success
                if e.to_string().contains("data did not match any variant") {
                    eprintln!("DEBUG: Skipping validation due to unknown values in config");
                } else {
                    return Err(e);
                }
            }
        }

        Ok(Response::new(validate_resource_config::Response {
            diagnostics: vec![],
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

        let provider = self.provider.lock().unwrap();
        let resources = provider.get_resources();

        let resource = resources
            .get(&type_name)
            .ok_or_else(|| Status::not_found(format!("Unknown resource type: {}", type_name)))?;

        let current_state = decode_dynamic_value(&req.current_state)?;
        let state = State {
            values: current_state.values,
        };

        match resource.read(state) {
            Ok((new_state, diags)) => {
                let (new_state_value, encoded_state) = match new_state {
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
                    diagnostics: encode_diagnostics(&diags),
                    private: encoded_state,
                    deferred: None,
                    new_identity: None,
                }))
            }
            Err(e) => {
                let mut diags = TfplugDiagnostics::new();
                diags.add_error(format!("Failed to read resource: {}", e), None::<String>);
                Ok(Response::new(read_resource::Response {
                    new_state: req.current_state.clone(),
                    diagnostics: encode_diagnostics(&diags),
                    private: vec![],
                    deferred: None,
                    new_identity: None,
                }))
            }
        }
    }

    async fn plan_resource_change(
        &self,
        request: Request<plan_resource_change::Request>,
    ) -> std::result::Result<Response<plan_resource_change::Response>, Status> {
        use crate::proto::tfplugin6::attribute_path::Step;
        use crate::proto::tfplugin6::AttributePath;

        let req = request.into_inner();
        let type_name = req.type_name;

        let provider = self.provider.lock().unwrap();
        let resources = provider.get_resources();

        let _resource = resources
            .get(&type_name)
            .ok_or_else(|| Status::not_found(format!("Unknown resource type: {}", type_name)))?;

        let prior_state = decode_dynamic_value(&req.prior_state)?.values;
        let config = decode_dynamic_value(&req.config)?.values;
        let proposed_new_state = decode_dynamic_value(&req.proposed_new_state)?.values;

        // For planning, we mostly just return the proposed state
        // Real validation happens during apply
        let planned_state = if prior_state.is_empty() && !proposed_new_state.is_empty() {
            // This is a create operation
            proposed_new_state
        } else if !prior_state.is_empty() && proposed_new_state.is_empty() {
            // This is a delete operation
            HashMap::new()
        } else {
            // This is an update operation
            proposed_new_state
        };

        // Check if we need to replace the resource (ForceNew attributes)
        let mut requires_replace = Vec::new();

        // For realm resource, "realm" and "type" are ForceNew
        if type_name == "proxmox_realm" {
            if let (Some(prior_realm), Some(new_realm)) = (
                prior_state.get("realm").and_then(|v| v.as_string()),
                config.get("realm").and_then(|v| v.as_string()),
            ) {
                if prior_realm != new_realm {
                    requires_replace.push(AttributePath {
                        steps: vec![Step {
                            selector: Some(crate::proto::tfplugin6::attribute_path::step::Selector::AttributeName("realm".to_string())),
                        }],
                    });
                }
            }

            if let (Some(prior_type), Some(new_type)) = (
                prior_state.get("type").and_then(|v| v.as_string()),
                config.get("type").and_then(|v| v.as_string()),
            ) {
                if prior_type != new_type {
                    requires_replace.push(AttributePath {
                        steps: vec![Step {
                            selector: Some(crate::proto::tfplugin6::attribute_path::step::Selector::AttributeName("type".to_string())),
                        }],
                    });
                }
            }
        }

        let encoded_planned_state = encode_dynamic_values(&planned_state)?;

        Ok(Response::new(plan_resource_change::Response {
            planned_state: Some(encoded_planned_state),
            requires_replace,
            planned_private: vec![],
            diagnostics: vec![],
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

        let provider = self.provider.lock().unwrap();
        let resources = provider.get_resources();

        let resource = resources
            .get(&type_name)
            .ok_or_else(|| Status::not_found(format!("Unknown resource type: {}", type_name)))?;

        let prior_state = decode_dynamic_value(&req.prior_state)?.values;
        let config = decode_dynamic_value(&req.config)?.values;
        let planned_state = decode_dynamic_value(&req.planned_state)?.values;

        // Determine the operation type
        let is_create = prior_state.is_empty() && !planned_state.is_empty();
        let is_delete = !prior_state.is_empty() && planned_state.is_empty();
        let is_update = !prior_state.is_empty() && !planned_state.is_empty();

        let result = if is_create {
            // Create operation
            let config = Config { values: config };
            resource.create(config)
        } else if is_delete {
            // Delete operation
            let state = State {
                values: prior_state.clone(),
            };
            match resource.delete(state) {
                Ok(diags) => Ok((
                    State {
                        values: HashMap::new(),
                    },
                    diags,
                )),
                Err(e) => Err(e),
            }
        } else if is_update {
            // Update operation
            let state = State {
                values: prior_state.clone(),
            };
            let cfg = Config {
                values: config.clone(),
            };
            resource.update(state, cfg)
        } else {
            // No-op
            Ok((
                State {
                    values: planned_state.clone(),
                },
                TfplugDiagnostics::new(),
            ))
        };

        match result {
            Ok((new_state, diags)) => {
                // For delete operations, return None for new_state
                let new_state_value = if is_delete && new_state.values.is_empty() {
                    None
                } else {
                    Some(encode_state(&new_state)?)
                };

                Ok(Response::new(apply_resource_change::Response {
                    new_state: new_state_value,
                    diagnostics: encode_diagnostics(&diags),
                    private: vec![],
                    legacy_type_system: false,
                    new_identity: None,
                }))
            }
            Err(e) => {
                let mut diags = TfplugDiagnostics::new();
                diags.add_error(
                    format!("Failed to apply resource change: {}", e),
                    None::<String>,
                );

                // For create operations that fail, we should return the planned state
                // so Terraform can retry. For other operations, return the prior state.
                let state_to_return = if is_create {
                    &planned_state
                } else {
                    &prior_state
                };

                Ok(Response::new(apply_resource_change::Response {
                    new_state: Some(encode_dynamic_values(state_to_return)?),
                    diagnostics: encode_diagnostics(&diags),
                    private: vec![],
                    legacy_type_system: false,
                    new_identity: None,
                }))
            }
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

        // Get the data source and immediately drop the provider lock
        // This prevents deadlock when data sources spawn threads (e.g., for async operations)
        // The lock must be released before calling read() to avoid holding it during I/O
        let data_source = {
            let provider = self.provider.lock().unwrap();
            let mut data_sources = provider.get_data_sources();

            eprintln!(
                "DEBUG: Available data sources: {:?}",
                data_sources.keys().collect::<Vec<_>>()
            );

            data_sources.remove(&type_name).ok_or_else(|| {
                Status::invalid_argument(format!("Unknown data source: {}", type_name))
            })?
        }; // Lock is dropped here

        eprintln!("DEBUG: Calling read on data source");

        let (state, diags) = data_source
            .read(config)
            .map_err(|e| Status::internal(format!("Failed to read data source: {}", e)))?;

        eprintln!("DEBUG: Read completed successfully");

        Ok(Response::new(read_data_source::Response {
            state: Some(encode_dynamic_value(&state)?),
            diagnostics: convert_diagnostics(diags),
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
        let upgraded_state = req.raw_state.as_ref().map(|raw| {
            DynamicValue {
                msgpack: vec![], // We'll use JSON for now
                json: raw.json.clone(),
            }
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

fn string_type() -> Vec<u8> {
    "\"string\"".as_bytes().to_vec()
}

fn bool_type() -> Vec<u8> {
    "\"bool\"".as_bytes().to_vec()
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
