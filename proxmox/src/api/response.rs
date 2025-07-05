//! Response handling utilities for Proxmox API

use super::ApiError;
use serde::de::DeserializeOwned;

#[allow(async_fn_in_trait)]
pub trait ResponseHandler {
    async fn handle_response<T: DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, ApiError>;

    async fn handle_empty_response(response: reqwest::Response) -> Result<(), ApiError>;
}

pub struct ProxmoxResponseHandler;

impl ProxmoxResponseHandler {
    pub async fn extract_response<T: DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, ApiError> {
        let status = response.status();
        let text = response.text().await?;

        if status.is_success() {
            match serde_json::from_str::<super::ApiResponse<T>>(&text) {
                Ok(wrapper) => Ok(wrapper.data),
                Err(_) => match serde_json::from_str::<T>(&text) {
                    Ok(data) => Ok(data),
                    Err(e) => {
                        tracing::error!("Failed to parse response: {}, body: {}", e, text);
                        Err(ApiError::ParseError(format!(
                            "Failed to parse response: {}",
                            e
                        )))
                    }
                },
            }
        } else {
            Self::extract_error(status.as_u16(), text).await
        }
    }

    pub async fn extract_empty_response(response: reqwest::Response) -> Result<(), ApiError> {
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Self::extract_error(status.as_u16(), text).await
        }
    }

    async fn extract_error<T>(status: u16, text: String) -> Result<T, ApiError> {
        if status == 401 {
            return Err(ApiError::AuthError);
        }

        if status == 429 {
            return Err(ApiError::RateLimited);
        }

        if status >= 500 {
            return Err(ApiError::ServiceUnavailable);
        }

        let details = match serde_json::from_str::<super::ApiErrorResponse>(&text) {
            Ok(err_resp) => Some(Box::new(super::ApiErrorDetails {
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
