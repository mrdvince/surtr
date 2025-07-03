use thiserror::Error;

use super::common::ApiErrorDetails;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("API returned error (HTTP {status}): {message}")]
    ApiError {
        status: u16,
        message: String,
        #[source]
        details: Option<Box<ApiErrorDetails>>,
    },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Authentication failed")]
    AuthError,

    #[error("Request timeout after {0} seconds")]
    Timeout(u64),

    #[error("Too many requests, rate limited")]
    RateLimited,

    #[error("Service unavailable, retry later")]
    ServiceUnavailable,
}
