use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::common::{ApiErrorDetails, ApiErrorResponse, ApiQueryParams, ApiResponse};
use super::error::ApiError;
use super::pool::{ConnectionPoolConfig, ConnectionPoolManager};

/// Proxmox API client
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    http_client: reqwest::Client,
    base_url: String,
    auth_header: String,
    retry_config: RetryConfig,
    pool_manager: ConnectionPoolManager,
}

#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub timeout_seconds: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10000,
            timeout_seconds: 30,
        }
    }
}

impl Client {
    /// Execute a GET request and expect no data wrapper
    pub async fn get_raw<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ApiError> {
        self.execute_with_retry(
            || async {
                let url = format!("{}{}", self.inner.base_url, path);

                tracing::debug!("GET request to: {}", url);

                self.inner
                    .http_client
                    .get(&url)
                    .header(AUTHORIZATION, &self.inner.auth_header)
                    .send()
                    .await
            },
            path,
        )
        .await
    }

    /// Execute a GET request with query parameters
    pub async fn get_with_params<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: &ApiQueryParams,
    ) -> Result<T, ApiError> {
        let full_path = format!("{}{}", path, params.to_query_string());
        self.get(&full_path).await
    }

    /// Create a new API client with default configuration
    pub fn new(endpoint: &str, api_token: &str, insecure: bool) -> Result<Self, ApiError> {
        Self::with_config(endpoint, api_token, insecure, RetryConfig::default())
    }

    /// Create a new API client with custom retry configuration
    pub fn with_config(
        endpoint: &str,
        api_token: &str,
        insecure: bool,
        retry_config: RetryConfig,
    ) -> Result<Self, ApiError> {
        let pool_config = ConnectionPoolConfig {
            request_timeout: std::time::Duration::from_secs(retry_config.timeout_seconds),
            ..Default::default()
        };

        let pool_manager = ConnectionPoolManager::new(pool_config);
        let http_client = pool_manager.build_client(insecure)?;

        let base_url = endpoint.trim_end_matches('/').to_string();
        let auth_header = format!("PVEAPIToken={}", api_token);

        Ok(Self {
            inner: Arc::new(ClientInner {
                http_client,
                base_url,
                auth_header,
                retry_config,
                pool_manager,
            }),
        })
    }

    /// Execute a GET request with retry logic
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ApiError> {
        self.execute_with_retry(
            || async {
                let url = format!("{}{}", self.inner.base_url, path);

                tracing::debug!("GET request to: {}", url);

                self.inner
                    .http_client
                    .get(&url)
                    .header(AUTHORIZATION, &self.inner.auth_header)
                    .send()
                    .await
            },
            path,
        )
        .await
    }

    /// Execute a POST request with retry logic
    pub async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let body_clone = body;
        self.execute_with_retry(
            || async {
                let url = format!("{}{}", self.inner.base_url, path);

                self.inner
                    .http_client
                    .post(&url)
                    .header(AUTHORIZATION, &self.inner.auth_header)
                    .json(body_clone)
                    .send()
                    .await
            },
            path,
        )
        .await
    }

    /// Execute a PUT request with retry logic
    pub async fn put<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let body_clone = body;
        self.execute_with_retry(
            || async {
                let url = format!("{}{}", self.inner.base_url, path);

                self.inner
                    .http_client
                    .put(&url)
                    .header(AUTHORIZATION, &self.inner.auth_header)
                    .json(body_clone)
                    .send()
                    .await
            },
            path,
        )
        .await
    }

    /// Get connection pool statistics
    pub async fn get_connection_stats(&self) -> super::pool::ConnectionStats {
        self.inner.pool_manager.get_stats().await
    }

    /// Access API operations
    pub fn access(&self) -> crate::api::access::AccessApi<'_> {
        crate::api::access::AccessApi::new(self)
    }

    /// Nodes API operations
    pub fn nodes(&self) -> crate::api::nodes::NodesApi<'_> {
        crate::api::nodes::NodesApi::new(self)
    }

    /// Execute a DELETE request with retry logic
    pub async fn delete<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ApiError> {
        self.execute_with_retry(
            || async {
                let url = format!("{}{}", self.inner.base_url, path);

                self.inner
                    .http_client
                    .delete(&url)
                    .header(AUTHORIZATION, &self.inner.auth_header)
                    .send()
                    .await
            },
            path,
        )
        .await
    }

    /// Execute request with retry logic
    async fn execute_with_retry<F, Fut, T>(&self, request_fn: F, path: &str) -> Result<T, ApiError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
        T: for<'de> Deserialize<'de>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= self.inner.retry_config.max_retries {
            if attempt > 0 {
                let backoff = std::cmp::min(
                    self.inner.retry_config.initial_backoff_ms * (2_u64.pow(attempt - 1)),
                    self.inner.retry_config.max_backoff_ms,
                );
                tracing::debug!(
                    "Retrying request to {} after {}ms (attempt {})",
                    path,
                    backoff,
                    attempt
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
            }

            match request_fn().await {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        self.inner.pool_manager.record_request(true).await;
                        return self.parse_success_response(response).await;
                    }

                    self.inner.pool_manager.record_request(false).await;

                    if status == reqwest::StatusCode::UNAUTHORIZED {
                        return Err(ApiError::AuthError);
                    }

                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        last_error = Some(ApiError::RateLimited);
                    } else if status.is_server_error() {
                        last_error = Some(ApiError::ServiceUnavailable);
                    } else {
                        return self.handle_error_response(response).await;
                    }
                }
                Err(e) => {
                    self.inner.pool_manager.record_request(false).await;

                    if e.is_timeout() {
                        last_error =
                            Some(ApiError::Timeout(self.inner.retry_config.timeout_seconds));
                    } else if e.is_connect() || e.is_request() {
                        last_error = Some(ApiError::ServiceUnavailable);
                    } else {
                        return Err(ApiError::RequestError(e));
                    }
                }
            }

            attempt += 1;
        }

        Err(last_error.unwrap_or(ApiError::ServiceUnavailable))
    }

    /// Parse successful response
    async fn parse_success_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, ApiError> {
        let text = response.text().await?;
        tracing::debug!("API response body: {}", text);

        match serde_json::from_str::<ApiResponse<T>>(&text) {
            Ok(wrapper) => Ok(wrapper.data),
            Err(_) => match serde_json::from_str::<T>(&text) {
                Ok(data) => Ok(data),
                Err(e) => {
                    tracing::error!("Failed to deserialize response: {}, body: {}", e, text);
                    Err(ApiError::ParseError(format!(
                        "Failed to parse response: {}",
                        e
                    )))
                }
            },
        }
    }

    /// Handle error response
    async fn handle_error_response<T>(&self, response: reqwest::Response) -> Result<T, ApiError> {
        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        let details = match serde_json::from_str::<ApiErrorResponse>(&text) {
            Ok(err_resp) => Some(Box::new(ApiErrorDetails {
                errors: err_resp.errors,
                field_errors: err_resp.data,
            })),
            Err(_) => None,
        };

        Err(ApiError::ApiError {
            status,
            message: text,
            details,
        })
    }
}
