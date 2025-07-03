//! Connection pool management for Proxmox API

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct ConnectionPoolConfig {
    pub max_idle_connections: usize,
    pub idle_timeout: Duration,
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
    pub tcp_keepalive: Option<Duration>,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_idle_connections: 10,
            idle_timeout: Duration::from_secs(90),
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            tcp_keepalive: Some(Duration::from_secs(30)),
        }
    }
}

#[derive(Default)]
pub struct ConnectionStats {
    pub total_requests: u64,
    pub failed_requests: u64,
    pub active_connections: usize,
    pub last_request: Option<Instant>,
}

pub struct ConnectionPoolManager {
    stats: Arc<RwLock<ConnectionStats>>,
    config: ConnectionPoolConfig,
}

impl ConnectionPoolManager {
    pub fn new(config: ConnectionPoolConfig) -> Self {
        Self {
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            config,
        }
    }

    pub async fn record_request(&self, success: bool) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        if !success {
            stats.failed_requests += 1;
        }
        stats.last_request = Some(Instant::now());
    }

    pub async fn get_stats(&self) -> ConnectionStats {
        let stats = self.stats.read().await;
        ConnectionStats {
            total_requests: stats.total_requests,
            failed_requests: stats.failed_requests,
            active_connections: stats.active_connections,
            last_request: stats.last_request,
        }
    }

    pub fn build_client(&self, insecure: bool) -> Result<reqwest::Client, reqwest::Error> {
        let mut builder = reqwest::Client::builder()
            .danger_accept_invalid_certs(insecure)
            .timeout(self.config.request_timeout)
            .connect_timeout(self.config.connection_timeout)
            .pool_idle_timeout(self.config.idle_timeout)
            .pool_max_idle_per_host(self.config.max_idle_connections);

        if let Some(keepalive) = self.config.tcp_keepalive {
            builder = builder.tcp_keepalive(keepalive);
        }

        builder.build()
    }
}
