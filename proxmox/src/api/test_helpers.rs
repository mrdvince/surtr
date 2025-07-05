//! Test helpers for the Proxmox API

#[cfg(test)]
#[allow(dead_code)]
pub fn create_test_client(url: &str) -> super::Client {
    super::Client::new(url, "test@pam!test=secret", true).unwrap()
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[tokio::test]
    async fn test_retry_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 100);
        assert_eq!(config.max_backoff_ms, 10000);
        assert_eq!(config.timeout_seconds, 30);
    }

    #[tokio::test]
    async fn test_proxmox_bool() {
        use common::ProxmoxBool;

        let b = ProxmoxBool::new(true);
        assert!(b.as_bool());

        let b = ProxmoxBool::from(false);
        assert!(!b.as_bool());

        let b: bool = ProxmoxBool(true).into();
        assert!(b);
    }

    #[tokio::test]
    async fn test_api_query_params() {
        use common::ApiQueryParams;

        let params = ApiQueryParams::new()
            .add("foo", "bar")
            .add("baz", 123)
            .add_optional("opt", Some("value"))
            .add_optional("none", None::<String>);

        let query = params.to_query_string();
        assert!(query.contains("foo=bar"));
        assert!(query.contains("baz=123"));
        assert!(query.contains("opt=value"));
        assert!(!query.contains("none="));
    }

    #[tokio::test]
    async fn test_pagination_params() {
        use common::PaginationParams;

        let params = PaginationParams::new().with_start(100).with_limit(50);

        let query_params = params.to_query_params();
        let query = query_params.to_query_string();

        assert!(query.contains("start=100"));
        assert!(query.contains("limit=50"));
    }

    #[tokio::test]
    async fn test_connection_pool_config() {
        use pool::ConnectionPoolConfig;

        let config = ConnectionPoolConfig::default();
        assert_eq!(config.max_idle_connections, 10);
        assert_eq!(config.idle_timeout.as_secs(), 90);
        assert_eq!(config.connection_timeout.as_secs(), 10);
        assert_eq!(config.request_timeout.as_secs(), 30);
        assert_eq!(config.tcp_keepalive.unwrap().as_secs(), 30);
    }

    #[tokio::test]
    async fn test_connection_stats() {
        use pool::{ConnectionPoolConfig, ConnectionPoolManager};

        let manager = ConnectionPoolManager::new(ConnectionPoolConfig::default());

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.failed_requests, 0);

        manager.record_request(true).await;
        manager.record_request(false).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.failed_requests, 1);
        assert!(stats.last_request.is_some());
    }

    #[test]
    fn test_api_error_formatting() {
        use std::collections::HashMap;

        let mut field_errors = HashMap::new();
        field_errors.insert(
            "field1".to_string(),
            vec!["error1".to_string(), "error2".to_string()],
        );

        let details = ApiErrorDetails {
            errors: Some(vec!["general error".to_string()]),
            field_errors: Some(field_errors),
        };

        let error = ApiError::ApiError {
            status: 400,
            message: "Bad Request".to_string(),
            details: Some(Box::new(details)),
        };

        let error_str = error.to_string();
        assert!(error_str.contains("HTTP 400"));
        assert!(error_str.contains("Bad Request"));
    }
}
