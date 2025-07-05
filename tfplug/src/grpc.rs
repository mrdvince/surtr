//! gRPC server implementation for the Terraform Plugin Protocol
//!
//! This module implements the gRPC server that communicates with Terraform/OpenTofu
//! using the Terraform Plugin Protocol v6.9.

use crate::context::Context;
use crate::proto;
use crate::provider::Provider;
use crate::types::DynamicValue;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

// Type alias to avoid clippy warning about large error types
type GrpcResult<T> = std::result::Result<T, Status>;

/// gRPC provider server that implements the Terraform Plugin Protocol
pub struct GrpcProviderServer<P: Provider> {
    provider: Arc<RwLock<P>>,
    provider_data: Arc<RwLock<Option<Arc<dyn std::any::Any + Send + Sync>>>>,
    configured: Arc<RwLock<bool>>,
}

impl<P: Provider + 'static> GrpcProviderServer<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(RwLock::new(provider)),
            provider_data: Arc::new(RwLock::new(None)),
            configured: Arc::new(RwLock::new(false)),
        }
    }
}

#[tonic::async_trait]
impl<P> proto::provider_server::Provider for GrpcProviderServer<P>
where
    P: Provider + 'static,
{
    async fn get_metadata(
        &self,
        _request: Request<proto::get_metadata::Request>,
    ) -> std::result::Result<Response<proto::get_metadata::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;

        let provider_response = provider
            .metadata(ctx.clone(), crate::provider::ProviderMetadataRequest)
            .await;

        let mut response = proto::get_metadata::Response {
            server_capabilities: Some(convert_server_capabilities(
                &provider_response.server_capabilities,
            )),
            diagnostics: vec![],
            resources: vec![],
            data_sources: vec![],
            ephemeral_resources: vec![],
            functions: vec![],
        };

        for (name, _) in provider.resources() {
            response
                .resources
                .push(proto::get_metadata::ResourceMetadata {
                    type_name: name.clone(),
                });
        }

        for (name, _) in provider.data_sources() {
            response
                .data_sources
                .push(proto::get_metadata::DataSourceMetadata {
                    type_name: name.clone(),
                });
        }

        // TODO: Handle functions and ephemeral resources when those traits are implemented

        Ok(Response::new(response))
    }

    async fn get_provider_schema(
        &self,
        _request: Request<proto::get_provider_schema::Request>,
    ) -> std::result::Result<Response<proto::get_provider_schema::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;

        let provider_schema_response = provider
            .schema(ctx.clone(), crate::provider::ProviderSchemaRequest)
            .await;

        let provider_meta_response = provider
            .meta_schema(ctx.clone(), crate::provider::ProviderMetaSchemaRequest)
            .await;

        let mut response = proto::get_provider_schema::Response {
            provider: Some(convert_schema(&provider_schema_response.schema)),
            provider_meta: provider_meta_response.schema.as_ref().map(convert_schema),
            resource_schemas: std::collections::HashMap::new(),
            data_source_schemas: std::collections::HashMap::new(),
            ephemeral_resource_schemas: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
            diagnostics: convert_diagnostics(&provider_schema_response.diagnostics),
            server_capabilities: Some(proto::ServerCapabilities {
                plan_destroy: false,
                get_provider_schema_optional: false,
                move_resource_state: false,
            }),
        };

        for (name, factory) in provider.resources() {
            let resource = factory();
            let schema_response = resource
                .schema(ctx.clone(), crate::resource::ResourceSchemaRequest)
                .await;
            response
                .resource_schemas
                .insert(name.clone(), convert_schema(&schema_response.schema));
        }

        for (name, factory) in provider.data_sources() {
            let data_source = factory();
            let schema_response = data_source
                .schema(ctx.clone(), crate::data_source::DataSourceSchemaRequest)
                .await;
            response
                .data_source_schemas
                .insert(name.clone(), convert_schema(&schema_response.schema));
        }

        Ok(Response::new(response))
    }

    async fn validate_provider_config(
        &self,
        request: Request<proto::validate_provider_config::Request>,
    ) -> std::result::Result<Response<proto::validate_provider_config::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let config = convert_dynamic_value_from_proto(
            &req.config
                .ok_or_else(|| Status::invalid_argument("config is required"))?,
        )?;

        let response = provider
            .validate(
                ctx,
                crate::provider::ValidateProviderConfigRequest {
                    config,
                    client_capabilities: crate::types::ClientCapabilities {
                        deferral_allowed: false,
                        write_only_attributes_allowed: false,
                    },
                },
            )
            .await;

        Ok(Response::new(proto::validate_provider_config::Response {
            diagnostics: convert_diagnostics(&response.diagnostics),
        }))
    }

    async fn configure_provider(
        &self,
        request: Request<proto::configure_provider::Request>,
    ) -> std::result::Result<Response<proto::configure_provider::Response>, Status> {
        let ctx = Context::new();
        let mut provider = self.provider.write().await;
        let req = request.into_inner();

        let config = convert_dynamic_value_from_proto(
            &req.config
                .ok_or_else(|| Status::invalid_argument("config is required"))?,
        )?;

        let response = provider
            .configure(
                ctx,
                crate::provider::ConfigureProviderRequest {
                    terraform_version: req.terraform_version,
                    config,
                    client_capabilities: convert_client_capabilities(&req.client_capabilities),
                },
            )
            .await;

        if let Some(data) = response.provider_data {
            tracing::debug!("ConfigureProvider: storing provider data");
            *self.provider_data.write().await = Some(data);
        } else {
            tracing::warn!("ConfigureProvider: no provider data returned");
        }
        *self.configured.write().await = true;

        Ok(Response::new(proto::configure_provider::Response {
            diagnostics: convert_diagnostics(&response.diagnostics),
        }))
    }

    async fn stop_provider(
        &self,
        _request: Request<proto::stop_provider::Request>,
    ) -> std::result::Result<Response<proto::stop_provider::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;

        let response = provider
            .stop(ctx, crate::provider::StopProviderRequest)
            .await;

        Ok(Response::new(proto::stop_provider::Response {
            error: response.error.unwrap_or_default(),
        }))
    }

    async fn validate_resource_config(
        &self,
        request: Request<proto::validate_resource_config::Request>,
    ) -> std::result::Result<Response<proto::validate_resource_config::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let resources = provider.resources();
        let factory = resources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("resource type '{}' not found", req.type_name))
        })?;

        let mut resource = factory();

        let provider_data = self.provider_data.read().await.clone();
        let _ = resource
            .configure(
                ctx.clone(),
                crate::resource::ConfigureResourceRequest { provider_data },
            )
            .await;

        let config = match convert_dynamic_value_from_proto(
            &req.config
                .ok_or_else(|| Status::invalid_argument("config is required"))?,
        ) {
            Ok(config) => config,
            Err(e) => {
                if e.message().contains("msgpack decoding failed") {
                    tracing::debug!("Skipping validation due to unknown values in config");
                    return Ok(Response::new(proto::validate_resource_config::Response {
                        diagnostics: vec![],
                    }));
                }
                return Err(e);
            }
        };

        let response = resource
            .validate(
                ctx,
                crate::resource::ValidateResourceConfigRequest {
                    type_name: req.type_name,
                    config,
                    client_capabilities: convert_client_capabilities(&req.client_capabilities),
                },
            )
            .await;

        Ok(Response::new(proto::validate_resource_config::Response {
            diagnostics: convert_diagnostics(&response.diagnostics),
        }))
    }

    async fn upgrade_resource_state(
        &self,
        request: Request<proto::upgrade_resource_state::Request>,
    ) -> std::result::Result<Response<proto::upgrade_resource_state::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let resources = provider.resources();
        let factory = resources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("resource type '{}' not found", req.type_name))
        })?;

        let mut resource = factory();

        let provider_data = self.provider_data.read().await.clone();
        let _ = resource
            .configure(
                ctx.clone(),
                crate::resource::ConfigureResourceRequest { provider_data },
            )
            .await;

        let schema_response = resource
            .schema(ctx.clone(), crate::resource::ResourceSchemaRequest)
            .await;

        let raw_state = req
            .raw_state
            .ok_or_else(|| Status::invalid_argument("raw_state is required"))?;

        if req.version == schema_response.schema.version {
            let state = if !raw_state.json.is_empty() {
                DynamicValue::decode_json(&raw_state.json).map_err(|e| {
                    Status::invalid_argument(format!("failed to decode state: {}", e))
                })?
            } else {
                return Err(Status::invalid_argument("raw state must have json"));
            };

            return Ok(Response::new(proto::upgrade_resource_state::Response {
                upgraded_state: Some(convert_dynamic_value_to_proto(&state)?),
                diagnostics: vec![],
            }));
        }

        {
            return Err(Status::unimplemented(format!(
                "resource '{}' schema version changed from {} to {} but ResourceWithUpgradeState not implemented",
                req.type_name, req.version, schema_response.schema.version
            )));
        }

        /* TODO: Enable when we have proper downcasting
        if let Some(upgradeable) = resource.as_any().downcast_ref::<dyn ResourceWithUpgradeState>() {
            let response = upgradeable
                .upgrade_state(
                    ctx,
                    crate::resource::UpgradeResourceStateRequest {
                        type_name: req.type_name,
                        version: req.version,
                        raw_state: convert_raw_state_from_proto(&raw_state),
                    },
                )
                .await;

            Ok(Response::new(proto::upgrade_resource_state::Response {
                upgraded_state: Some(convert_dynamic_value_to_proto(&response.upgraded_state)?),
                diagnostics: convert_diagnostics(&response.diagnostics),
            }))
        }
        */
    }

    async fn read_resource(
        &self,
        request: Request<proto::read_resource::Request>,
    ) -> std::result::Result<Response<proto::read_resource::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let resources = provider.resources();
        let factory = resources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("resource type '{}' not found", req.type_name))
        })?;

        let mut resource = factory();

        let provider_data = self.provider_data.read().await.clone();
        let _ = resource
            .configure(
                ctx.clone(),
                crate::resource::ConfigureResourceRequest { provider_data },
            )
            .await;

        let current_state = convert_dynamic_value_from_proto(
            &req.current_state
                .ok_or_else(|| Status::invalid_argument("current_state is required"))?,
        )?;

        let private = if req.private.is_empty() {
            vec![]
        } else {
            req.private
        };

        let response = resource
            .read(
                ctx,
                crate::resource::ReadResourceRequest {
                    type_name: req.type_name,
                    current_state,
                    private,
                    provider_meta: req
                        .provider_meta
                        .as_ref()
                        .map(convert_dynamic_value_from_proto)
                        .transpose()?,
                    client_capabilities: convert_client_capabilities(&req.client_capabilities),
                    current_identity: None, // TODO: Handle identity when implemented
                },
            )
            .await;

        Ok(Response::new(proto::read_resource::Response {
            new_state: response
                .new_state
                .as_ref()
                .map(convert_dynamic_value_to_proto)
                .transpose()?,
            diagnostics: convert_diagnostics(&response.diagnostics),
            private: response.private,
            deferred: response.deferred.as_ref().map(convert_deferred),
            new_identity: None, // TODO: Handle identity when implemented
        }))
    }

    async fn plan_resource_change(
        &self,
        request: Request<proto::plan_resource_change::Request>,
    ) -> std::result::Result<Response<proto::plan_resource_change::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let resources = provider.resources();
        let factory = resources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("resource type '{}' not found", req.type_name))
        })?;

        let mut resource = factory();

        let provider_data = self.provider_data.read().await.clone();
        let _ = resource
            .configure(
                ctx.clone(),
                crate::resource::ConfigureResourceRequest { provider_data },
            )
            .await;

        let _config = convert_dynamic_value_from_proto(
            &req.config
                .ok_or_else(|| Status::invalid_argument("config is required"))?,
        )?;

        let _prior_state = convert_dynamic_value_from_proto(
            &req.prior_state
                .ok_or_else(|| Status::invalid_argument("prior_state is required"))?,
        )?;

        let proposed_new_state = convert_dynamic_value_from_proto(
            &req.proposed_new_state
                .ok_or_else(|| Status::invalid_argument("proposed_new_state is required"))?,
        )?;

        let planned_state = proposed_new_state.clone();
        let requires_replace = vec![];
        let planned_private = req.prior_private.clone();
        let diagnostics = vec![];

        // If resource implements ModifyPlan, call it
        // TODO: Implement proper downcasting when we have a better type system
        /* TODO: Enable when we have proper downcasting
        if let Some(plan_modifier) = resource.as_any().downcast_ref::<dyn ResourceWithModifyPlan>() {
            let modify_response = plan_modifier
                .modify_plan(
                    ctx,
                    crate::resource::ModifyPlanRequest {
                        type_name: req.type_name.clone(),
                        config: config.clone(),
                        prior_state,
                        proposed_new_state: proposed_new_state.clone(),
                        prior_private: req.prior_private.clone(),
                        provider_meta: req.provider_meta.as_ref().map(convert_dynamic_value_from_proto).transpose()?,
                    },
                )
                .await;

            planned_state = modify_response.planned_state;
            requires_replace = modify_response.requires_replace;
            planned_private = modify_response.planned_private;
            diagnostics = modify_response.diagnostics;
        }
        */

        Ok(Response::new(proto::plan_resource_change::Response {
            planned_state: Some(convert_dynamic_value_to_proto(&planned_state)?),
            requires_replace: convert_attribute_paths(&requires_replace),
            planned_private,
            diagnostics: convert_diagnostics(&diagnostics),
            legacy_type_system: false,
            deferred: None,
            planned_identity: None, // TODO: Handle identity when implemented
        }))
    }

    async fn apply_resource_change(
        &self,
        request: Request<proto::apply_resource_change::Request>,
    ) -> std::result::Result<Response<proto::apply_resource_change::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let resources = provider.resources();
        let factory = resources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("resource type '{}' not found", req.type_name))
        })?;

        let mut resource = factory();

        let provider_data = self.provider_data.read().await.clone();
        let _ = resource
            .configure(
                ctx.clone(),
                crate::resource::ConfigureResourceRequest { provider_data },
            )
            .await;

        let config = convert_dynamic_value_from_proto(
            &req.config
                .ok_or_else(|| Status::invalid_argument("config is required"))?,
        )?;

        let prior_state = req
            .prior_state
            .as_ref()
            .map(convert_dynamic_value_from_proto)
            .transpose()?;
        let planned_state = req
            .planned_state
            .as_ref()
            .map(convert_dynamic_value_from_proto)
            .transpose()?;

        let is_create = prior_state.as_ref().map(|s| s.is_null()).unwrap_or(true);
        let is_delete = planned_state.as_ref().map(|s| s.is_null()).unwrap_or(true);

        let response = if is_create && !is_delete {
            let create_response = resource
                .create(
                    ctx,
                    crate::resource::CreateResourceRequest {
                        type_name: req.type_name,
                        planned_state: planned_state.unwrap_or_else(DynamicValue::null),
                        config,
                        planned_private: req.planned_private,
                        provider_meta: req
                            .provider_meta
                            .as_ref()
                            .map(convert_dynamic_value_from_proto)
                            .transpose()?,
                    },
                )
                .await;

            proto::apply_resource_change::Response {
                new_state: Some(convert_dynamic_value_to_proto(&create_response.new_state)?),
                private: create_response.private,
                diagnostics: convert_diagnostics(&create_response.diagnostics),
                legacy_type_system: false,
                new_identity: None, // TODO: Handle identity when implemented
            }
        } else if !is_create && is_delete {
            let delete_response = resource
                .delete(
                    ctx,
                    crate::resource::DeleteResourceRequest {
                        type_name: req.type_name,
                        prior_state: prior_state.unwrap_or_else(DynamicValue::null),
                        planned_private: req.planned_private,
                        provider_meta: req
                            .provider_meta
                            .as_ref()
                            .map(convert_dynamic_value_from_proto)
                            .transpose()?,
                    },
                )
                .await;

            proto::apply_resource_change::Response {
                new_state: None,
                private: vec![],
                diagnostics: convert_diagnostics(&delete_response.diagnostics),
                legacy_type_system: false,
                new_identity: None, // TODO: Handle identity when implemented
            }
        } else if !is_create && !is_delete {
            let update_response = resource
                .update(
                    ctx,
                    crate::resource::UpdateResourceRequest {
                        type_name: req.type_name,
                        prior_state: prior_state.unwrap_or_else(DynamicValue::null),
                        planned_state: planned_state.unwrap_or_else(DynamicValue::null),
                        config,
                        planned_private: req.planned_private,
                        provider_meta: req
                            .provider_meta
                            .as_ref()
                            .map(convert_dynamic_value_from_proto)
                            .transpose()?,
                        planned_identity: None, // TODO: Handle identity when implemented
                    },
                )
                .await;

            proto::apply_resource_change::Response {
                new_state: Some(convert_dynamic_value_to_proto(&update_response.new_state)?),
                private: update_response.private,
                diagnostics: convert_diagnostics(&update_response.diagnostics),
                legacy_type_system: false,
                new_identity: None, // TODO: Handle identity when implemented
            }
        } else {
            return Err(Status::invalid_argument(
                "invalid state combination for apply",
            ));
        };

        Ok(Response::new(response))
    }

    async fn import_resource_state(
        &self,
        request: Request<proto::import_resource_state::Request>,
    ) -> std::result::Result<Response<proto::import_resource_state::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let resources = provider.resources();
        let factory = resources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("resource type '{}' not found", req.type_name))
        })?;

        let mut resource = factory();

        let provider_data = self.provider_data.read().await.clone();
        let _ = resource
            .configure(
                ctx.clone(),
                crate::resource::ConfigureResourceRequest { provider_data },
            )
            .await;

        /* TODO: Enable when we have proper downcasting
        if let Some(importable) = resource.as_any().downcast_ref::<dyn ResourceWithImportState>() {
            let response = importable
                .import_state(
                    ctx,
                    crate::resource::ImportResourceStateRequest {
                        type_name: req.type_name,
                        id: req.id,
                        client_capabilities: convert_client_capabilities(&req.client_capabilities),
                        identity: None, // TODO: Handle identity when implemented
                    },
                )
                .await;

            let imported_resources = response.imported_resources.iter()
                .map(|r| {
                    Ok(proto::import_resource_state::ImportedResource {
                        type_name: r.type_name.clone(),
                        state: Some(convert_dynamic_value_to_proto(&r.state)?),
                        private: r.private.clone(),
                        identity: None, // TODO: Handle identity when implemented
                    })
                })
                .collect::<Result<Vec<_>, Status>>()?;

            Ok(Response::new(proto::import_resource_state::Response {
                imported_resources,
                diagnostics: convert_diagnostics(&response.diagnostics),
                deferred: response.deferred.as_ref().map(convert_deferred),
            }))
        } else {
        */
        return Err(Status::unimplemented(format!(
            "resource '{}' does not implement import",
            req.type_name
        )));
        /*
        }
        */
    }

    async fn move_resource_state(
        &self,
        _request: Request<proto::move_resource_state::Request>,
    ) -> std::result::Result<Response<proto::move_resource_state::Response>, Status> {
        // TODO: Implement when ResourceWithMoveState trait is available
        Err(Status::unimplemented("move_resource_state not implemented"))
    }

    async fn read_data_source(
        &self,
        request: Request<proto::read_data_source::Request>,
    ) -> std::result::Result<Response<proto::read_data_source::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let data_sources = provider.data_sources();
        let factory = data_sources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("data source type '{}' not found", req.type_name))
        })?;

        let mut data_source = factory();

        let provider_data = self.provider_data.read().await.clone();
        tracing::debug!(
            "ReadDataSource: provider_data available: {}",
            provider_data.is_some()
        );
        let _ = data_source
            .configure(
                ctx.clone(),
                crate::data_source::ConfigureDataSourceRequest { provider_data },
            )
            .await;

        let config = convert_dynamic_value_from_proto(
            &req.config
                .ok_or_else(|| Status::invalid_argument("config is required"))?,
        )?;

        let response = data_source
            .read(
                ctx,
                crate::data_source::ReadDataSourceRequest {
                    type_name: req.type_name,
                    config,
                    provider_meta: req
                        .provider_meta
                        .as_ref()
                        .map(convert_dynamic_value_from_proto)
                        .transpose()?,
                    client_capabilities: convert_client_capabilities(&req.client_capabilities),
                },
            )
            .await;

        Ok(Response::new(proto::read_data_source::Response {
            state: Some(convert_dynamic_value_to_proto(&response.state)?),
            diagnostics: convert_diagnostics(&response.diagnostics),
            deferred: response.deferred.as_ref().map(convert_deferred),
        }))
    }

    async fn validate_data_resource_config(
        &self,
        request: Request<proto::validate_data_resource_config::Request>,
    ) -> std::result::Result<Response<proto::validate_data_resource_config::Response>, Status> {
        let ctx = Context::new();
        let provider = self.provider.read().await;
        let req = request.into_inner();

        let data_sources = provider.data_sources();
        let factory = data_sources.get(&req.type_name).ok_or_else(|| {
            Status::not_found(format!("data source type '{}' not found", req.type_name))
        })?;

        let mut data_source = factory();

        let provider_data = self.provider_data.read().await.clone();
        let _ = data_source
            .configure(
                ctx.clone(),
                crate::data_source::ConfigureDataSourceRequest { provider_data },
            )
            .await;

        let config = match convert_dynamic_value_from_proto(
            &req.config
                .ok_or_else(|| Status::invalid_argument("config is required"))?,
        ) {
            Ok(config) => config,
            Err(e) => {
                if e.message().contains("msgpack decoding failed") {
                    tracing::debug!("Skipping validation due to unknown values in config");
                    return Ok(Response::new(
                        proto::validate_data_resource_config::Response {
                            diagnostics: vec![],
                        },
                    ));
                }
                return Err(e);
            }
        };

        let response = data_source
            .validate(
                ctx,
                crate::data_source::ValidateDataSourceConfigRequest {
                    type_name: req.type_name,
                    config,
                },
            )
            .await;

        Ok(Response::new(
            proto::validate_data_resource_config::Response {
                diagnostics: convert_diagnostics(&response.diagnostics),
            },
        ))
    }

    async fn get_functions(
        &self,
        _request: Request<proto::get_functions::Request>,
    ) -> std::result::Result<Response<proto::get_functions::Response>, Status> {
        // TODO: Implement when ProviderWithFunctions trait is available
        Ok(Response::new(proto::get_functions::Response {
            functions: std::collections::HashMap::new(),
            diagnostics: vec![],
        }))
    }

    async fn call_function(
        &self,
        _request: Request<proto::call_function::Request>,
    ) -> std::result::Result<Response<proto::call_function::Response>, Status> {
        // TODO: Implement when Function trait is available
        Err(Status::unimplemented("call_function not implemented"))
    }

    async fn validate_ephemeral_resource_config(
        &self,
        _request: Request<proto::validate_ephemeral_resource_config::Request>,
    ) -> std::result::Result<Response<proto::validate_ephemeral_resource_config::Response>, Status>
    {
        // TODO: Implement when EphemeralResource trait is available
        Err(Status::unimplemented(
            "validate_ephemeral_resource_config not implemented",
        ))
    }

    async fn open_ephemeral_resource(
        &self,
        _request: Request<proto::open_ephemeral_resource::Request>,
    ) -> std::result::Result<Response<proto::open_ephemeral_resource::Response>, Status> {
        // TODO: Implement when EphemeralResource trait is available
        Err(Status::unimplemented(
            "open_ephemeral_resource not implemented",
        ))
    }

    async fn renew_ephemeral_resource(
        &self,
        _request: Request<proto::renew_ephemeral_resource::Request>,
    ) -> std::result::Result<Response<proto::renew_ephemeral_resource::Response>, Status> {
        // TODO: Implement when EphemeralResource trait is available
        Err(Status::unimplemented(
            "renew_ephemeral_resource not implemented",
        ))
    }

    async fn close_ephemeral_resource(
        &self,
        _request: Request<proto::close_ephemeral_resource::Request>,
    ) -> std::result::Result<Response<proto::close_ephemeral_resource::Response>, Status> {
        // TODO: Implement when EphemeralResource trait is available
        Err(Status::unimplemented(
            "close_ephemeral_resource not implemented",
        ))
    }

    async fn upgrade_resource_identity(
        &self,
        _request: Request<proto::upgrade_resource_identity::Request>,
    ) -> std::result::Result<Response<proto::upgrade_resource_identity::Response>, Status> {
        // TODO: Implement when ResourceWithUpgradeIdentity trait is available
        Err(Status::unimplemented(
            "upgrade_resource_identity not implemented",
        ))
    }

    async fn get_resource_identity_schemas(
        &self,
        _request: Request<proto::get_resource_identity_schemas::Request>,
    ) -> std::result::Result<Response<proto::get_resource_identity_schemas::Response>, Status> {
        // TODO: Implement when ResourceWithIdentity trait is available
        Ok(Response::new(
            proto::get_resource_identity_schemas::Response {
                identity_schemas: std::collections::HashMap::new(),
                diagnostics: vec![],
            },
        ))
    }
}

// Conversion functions

fn convert_server_capabilities(
    caps: &crate::types::ServerCapabilities,
) -> proto::ServerCapabilities {
    proto::ServerCapabilities {
        plan_destroy: caps.plan_destroy,
        get_provider_schema_optional: caps.get_provider_schema_optional,
        move_resource_state: caps.move_resource_state,
    }
}

fn convert_client_capabilities(
    caps: &Option<proto::ClientCapabilities>,
) -> crate::types::ClientCapabilities {
    caps.as_ref()
        .map(|c| crate::types::ClientCapabilities {
            deferral_allowed: c.deferral_allowed,
            write_only_attributes_allowed: c.write_only_attributes_allowed,
        })
        .unwrap_or(crate::types::ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        })
}

fn convert_schema(schema: &crate::schema::Schema) -> proto::Schema {
    proto::Schema {
        version: schema.version,
        block: Some(convert_block(&schema.block)),
    }
}

fn convert_block(block: &crate::schema::Block) -> proto::schema::Block {
    proto::schema::Block {
        version: block.version,
        attributes: block.attributes.iter().map(convert_attribute).collect(),
        block_types: block.block_types.iter().map(convert_nested_block).collect(),
        description: block.description.clone(),
        description_kind: convert_string_kind(block.description_kind) as i32,
        deprecated: block.deprecated,
    }
}

fn convert_attribute(attr: &crate::schema::Attribute) -> proto::schema::Attribute {
    use crate::schema::AttributeType;

    // Convert the attribute type to proto bytes
    let type_bytes = match &attr.r#type {
        AttributeType::String => b"\"string\"".to_vec(),
        AttributeType::Number => b"\"number\"".to_vec(),
        AttributeType::Bool => b"\"bool\"".to_vec(),
        AttributeType::List(inner) => {
            let inner_type = match inner.as_ref() {
                AttributeType::String => "\"string\"",
                AttributeType::Number => "\"number\"",
                AttributeType::Bool => "\"bool\"",
                _ => "\"dynamic\"", // For complex types
            };
            format!("[\"list\", {}]", inner_type).into_bytes()
        }
        AttributeType::Set(inner) => {
            let inner_type = match inner.as_ref() {
                AttributeType::String => "\"string\"",
                AttributeType::Number => "\"number\"",
                AttributeType::Bool => "\"bool\"",
                _ => "\"dynamic\"", // For complex types
            };
            format!("[\"set\", {}]", inner_type).into_bytes()
        }
        AttributeType::Map(inner) => {
            let inner_type = match inner.as_ref() {
                AttributeType::String => "\"string\"",
                AttributeType::Number => "\"number\"",
                AttributeType::Bool => "\"bool\"",
                _ => "\"dynamic\"", // For complex types
            };
            format!("[\"map\", {}]", inner_type).into_bytes()
        }
        AttributeType::Object(_) => {
            // For objects, we'll use dynamic type for now
            b"\"dynamic\"".to_vec()
        }
    };

    proto::schema::Attribute {
        name: attr.name.clone(),
        r#type: type_bytes,
        nested_type: attr.nested_type.as_ref().map(convert_nested_type),
        description: attr.description.clone(),
        required: attr.required,
        optional: attr.optional,
        computed: attr.computed,
        sensitive: attr.sensitive,
        description_kind: proto::StringKind::Plain as i32,
        deprecated: attr.deprecated,
        write_only: false,
    }
}

#[allow(deprecated)]
fn convert_nested_type(nested: &crate::schema::NestedType) -> proto::schema::Object {
    proto::schema::Object {
        attributes: nested.attributes.iter().map(convert_attribute).collect(),
        nesting: convert_object_nesting_mode(nested.nesting) as i32,
        min_items: 0,
        max_items: 0,
    }
}

fn convert_nested_block(nested: &crate::schema::NestedBlock) -> proto::schema::NestedBlock {
    proto::schema::NestedBlock {
        type_name: nested.type_name.clone(),
        block: Some(convert_block(&nested.block)),
        nesting: convert_nesting_mode(nested.nesting) as i32,
        min_items: nested.min_items,
        max_items: nested.max_items,
    }
}

fn convert_string_kind(kind: crate::schema::StringKind) -> proto::StringKind {
    match kind {
        crate::schema::StringKind::Plain => proto::StringKind::Plain,
        crate::schema::StringKind::Markdown => proto::StringKind::Markdown,
    }
}

fn convert_nesting_mode(
    mode: crate::schema::NestingMode,
) -> proto::schema::nested_block::NestingMode {
    use crate::schema::NestingMode;

    match mode {
        NestingMode::Invalid => proto::schema::nested_block::NestingMode::Invalid,
        NestingMode::Single => proto::schema::nested_block::NestingMode::Single,
        NestingMode::List => proto::schema::nested_block::NestingMode::List,
        NestingMode::Set => proto::schema::nested_block::NestingMode::Set,
        NestingMode::Map => proto::schema::nested_block::NestingMode::Map,
        NestingMode::Group => proto::schema::nested_block::NestingMode::Group,
    }
}

fn convert_object_nesting_mode(
    mode: crate::schema::ObjectNestingMode,
) -> proto::schema::object::NestingMode {
    use crate::schema::ObjectNestingMode;

    match mode {
        ObjectNestingMode::Invalid => proto::schema::object::NestingMode::Invalid,
        ObjectNestingMode::Single => proto::schema::object::NestingMode::Single,
        ObjectNestingMode::List => proto::schema::object::NestingMode::List,
        ObjectNestingMode::Set => proto::schema::object::NestingMode::Set,
        ObjectNestingMode::Map => proto::schema::object::NestingMode::Map,
    }
}

fn convert_diagnostics(diags: &[crate::types::Diagnostic]) -> Vec<proto::Diagnostic> {
    diags.iter().map(convert_diagnostic).collect()
}

fn convert_diagnostic(diag: &crate::types::Diagnostic) -> proto::Diagnostic {
    proto::Diagnostic {
        severity: convert_diagnostic_severity(diag.severity) as i32,
        summary: diag.summary.clone(),
        detail: diag.detail.clone(),
        attribute: diag.attribute.as_ref().map(convert_attribute_path),
    }
}

fn convert_diagnostic_severity(
    severity: crate::types::DiagnosticSeverity,
) -> proto::diagnostic::Severity {
    use crate::types::DiagnosticSeverity;
    use proto::diagnostic::Severity;

    match severity {
        DiagnosticSeverity::Invalid => Severity::Invalid,
        DiagnosticSeverity::Error => Severity::Error,
        DiagnosticSeverity::Warning => Severity::Warning,
    }
}

fn convert_attribute_path(path: &crate::types::AttributePath) -> proto::AttributePath {
    proto::AttributePath {
        steps: path.steps.iter().map(convert_attribute_path_step).collect(),
    }
}

fn convert_attribute_path_step(
    step: &crate::types::AttributePathStep,
) -> proto::attribute_path::Step {
    use crate::types::AttributePathStep;
    use proto::attribute_path::step::Selector;

    let selector = match step {
        AttributePathStep::AttributeName(name) => Selector::AttributeName(name.clone()),
        AttributePathStep::ElementKeyString(key) => Selector::ElementKeyString(key.clone()),
        AttributePathStep::ElementKeyInt(idx) => Selector::ElementKeyInt(*idx),
    };

    proto::attribute_path::Step {
        selector: Some(selector),
    }
}

fn convert_attribute_paths(paths: &[crate::types::AttributePath]) -> Vec<proto::AttributePath> {
    paths.iter().map(convert_attribute_path).collect()
}

#[allow(clippy::result_large_err)]
fn convert_dynamic_value_from_proto(proto_val: &proto::DynamicValue) -> GrpcResult<DynamicValue> {
    if !proto_val.msgpack.is_empty() {
        tracing::debug!("Decoding msgpack of {} bytes", proto_val.msgpack.len());
        let preview_len = proto_val.msgpack.len().min(100);
        tracing::debug!(
            "First {} bytes: {:?}",
            preview_len,
            &proto_val.msgpack[..preview_len]
        );

        DynamicValue::decode_msgpack(&proto_val.msgpack)
            .map_err(|e| Status::invalid_argument(format!("failed to decode msgpack: {}", e)))
    } else if !proto_val.json.is_empty() {
        DynamicValue::decode_json(&proto_val.json)
            .map_err(|e| Status::invalid_argument(format!("failed to decode json: {}", e)))
    } else {
        Ok(DynamicValue::null())
    }
}

#[allow(clippy::result_large_err)]
fn convert_dynamic_value_to_proto(val: &DynamicValue) -> GrpcResult<proto::DynamicValue> {
    let msgpack = val
        .encode_msgpack()
        .map_err(|e| Status::internal(format!("failed to encode msgpack: {}", e)))?;

    Ok(proto::DynamicValue {
        msgpack,
        json: vec![],
    })
}

#[allow(dead_code)]
fn convert_raw_state_from_proto(proto_state: &proto::RawState) -> crate::types::RawState {
    crate::types::RawState {
        json: if proto_state.json.is_empty() {
            None
        } else {
            Some(proto_state.json.clone())
        },
        flatmap: if proto_state.flatmap.is_empty() {
            None
        } else {
            Some(proto_state.flatmap.clone())
        },
    }
}

fn convert_deferred(deferred: &crate::types::Deferred) -> proto::Deferred {
    proto::Deferred {
        reason: convert_deferred_reason(deferred.reason) as i32,
    }
}

fn convert_deferred_reason(reason: crate::types::DeferredReason) -> proto::deferred::Reason {
    use crate::types::DeferredReason;
    use proto::deferred::Reason;

    match reason {
        DeferredReason::Unknown => Reason::Unknown,
        DeferredReason::ResourceConfigUnknown => Reason::ResourceConfigUnknown,
        DeferredReason::ProviderConfigUnknown => Reason::ProviderConfigUnknown,
        DeferredReason::AbsentPrereq => Reason::AbsentPrereq,
    }
}
