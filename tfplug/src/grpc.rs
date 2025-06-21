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

        Ok(Response::new(get_provider_schema::Response {
            provider: Some(provider_schema),
            resource_schemas: HashMap::new(),
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
        _request: Request<validate_resource_config::Request>,
    ) -> std::result::Result<Response<validate_resource_config::Response>, Status> {
        todo!()
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
        _request: Request<read_resource::Request>,
    ) -> std::result::Result<Response<read_resource::Response>, Status> {
        todo!()
    }

    async fn plan_resource_change(
        &self,
        _request: Request<plan_resource_change::Request>,
    ) -> std::result::Result<Response<plan_resource_change::Response>, Status> {
        todo!()
    }

    async fn apply_resource_change(
        &self,
        _request: Request<apply_resource_change::Request>,
    ) -> std::result::Result<Response<apply_resource_change::Response>, Status> {
        todo!()
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

        let provider = self.provider.lock().unwrap();
        let data_sources = provider.get_data_sources();

        eprintln!(
            "DEBUG: Available data sources: {:?}",
            data_sources.keys().collect::<Vec<_>>()
        );

        let data_source = data_sources.get(&type_name).ok_or_else(|| {
            Status::invalid_argument(format!("Unknown data source: {}", type_name))
        })?;

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
        _request: Request<upgrade_resource_state::Request>,
    ) -> std::result::Result<Response<upgrade_resource_state::Response>, Status> {
        todo!()
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
    let value = value
        .as_ref()
        .ok_or_else(|| Status::invalid_argument("Missing value"))?;

    if !value.msgpack.is_empty() {
        let values: HashMap<String, Dynamic> = decode::from_slice(&value.msgpack)
            .map_err(|e| Status::invalid_argument(format!("Failed to decode msgpack: {}", e)))?;
        Ok(Config { values })
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
