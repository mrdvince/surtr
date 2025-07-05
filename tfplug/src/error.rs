//! Error types for tfplug

/// Error type for tfplug operations
#[derive(Debug, thiserror::Error)]
pub enum TfplugError {
    #[error("Resource type not found: {0}")]
    ResourceNotFound(String),

    #[error("Data source type not found: {0}")]
    DataSourceNotFound(String),

    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    #[error("Ephemeral resource type not found: {0}")]
    EphemeralResourceNotFound(String),

    #[error("Provider not configured")]
    ProviderNotConfigured,

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Import failed: {0}")]
    ImportFailed(String),

    #[error("Upgrade failed: {0}")]
    UpgradeFailed(String),

    #[error("gRPC error: {0}")]
    GrpcError(Box<tonic::Status>),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("TLS configuration error: {0}")]
    TlsError(String),

    #[error("Address parse error: {0}")]
    AddressParseError(#[from] std::net::AddrParseError),

    #[error("Transport error: {0}")]
    TransportError(#[from] tonic::transport::Error),

    #[error("{0}")]
    Custom(String),
}

/// Result type alias for tfplug operations
pub type Result<T> = std::result::Result<T, TfplugError>;

impl From<String> for TfplugError {
    fn from(s: String) -> Self {
        TfplugError::Custom(s)
    }
}

impl From<&str> for TfplugError {
    fn from(s: &str) -> Self {
        TfplugError::Custom(s.to_string())
    }
}

impl From<tonic::Status> for TfplugError {
    fn from(status: tonic::Status) -> Self {
        TfplugError::GrpcError(Box::new(status))
    }
}
