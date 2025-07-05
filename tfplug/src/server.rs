//! Server module for running Terraform providers
//!
//! This module provides functionality to start a Terraform provider server
//! with TLS support and proper protocol handshake.

use crate::error::{Result, TfplugError};
use crate::grpc::GrpcProviderServer;
use crate::proto::provider_server::ProviderServer;
use crate::provider::Provider;
use std::path::PathBuf;
use std::time::Duration;
use tonic::transport::{Identity, Server, ServerTlsConfig};

/// Log level for the server
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Server configuration for running a Terraform provider
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Path to TLS certificate file
    pub cert_path: PathBuf,
    /// Path to TLS key file
    pub key_path: PathBuf,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Whether to enable logging
    pub enable_logging: bool,
    /// Log level
    pub log_level: LogLevel,
    /// Timeout for graceful shutdown
    pub shutdown_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::from("./certs/localhost.pem"),
            key_path: PathBuf::from("./certs/localhost-key.pem"),
            max_message_size: 256 << 20, // 256MB
            enable_logging: true,
            log_level: LogLevel::Info,
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

impl ServerConfig {
    /// Create a new server configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the certificate path
    pub fn with_cert_path(mut self, path: PathBuf) -> Self {
        self.cert_path = path;
        self
    }

    /// Set the key path
    pub fn with_key_path(mut self, path: PathBuf) -> Self {
        self.key_path = path;
        self
    }

    /// Set the maximum message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Disable logging
    pub fn without_logging(mut self) -> Self {
        self.enable_logging = false;
        self
    }

    /// Set the log level
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Set the shutdown timeout
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }
}

/// Main entry point for running a provider
pub async fn serve<P: Provider + 'static>(provider: P, config: ServerConfig) -> Result<()> {
    // Initialize logging if enabled
    if config.enable_logging {
        // Logging initialization would go here
    }

    // Create the gRPC server
    let grpc_server = GrpcProviderServer::new(provider);
    let provider_service = ProviderServer::new(grpc_server)
        .max_decoding_message_size(config.max_message_size)
        .max_encoding_message_size(config.max_message_size);

    // Load TLS configuration
    let cert = tokio::fs::read(&config.cert_path)
        .await
        .map_err(|e| TfplugError::TlsError(format!("Failed to read certificate: {}", e)))?;

    let key = tokio::fs::read(&config.key_path)
        .await
        .map_err(|e| TfplugError::TlsError(format!("Failed to read key: {}", e)))?;

    let identity = Identity::from_pem(cert, key);
    let tls_config = ServerTlsConfig::new().identity(identity);

    // Create a TCP listener
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let actual_addr = listener.local_addr()?;

    println!("1|6|tcp|{}|grpc", actual_addr);

    // Create the server with TLS
    let server = Server::builder()
        .tls_config(tls_config)?
        .add_service(provider_service);

    // Run the server with the listener
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
    server.serve_with_incoming(incoming).await?;

    Ok(())
}

/// Convenience function to run a provider with default configuration
pub async fn serve_default<P: Provider + 'static>(provider: P) -> Result<()> {
    serve(provider, ServerConfig::default()).await
}
