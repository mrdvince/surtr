//! Context implementation for request-scoped data and cancellation
//!
//! This module provides the Context type which carries request-scoped values
//! like cancellation signals, timeouts, and metadata across async boundaries.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{watch, RwLock};
use tokio::time;

/// Context carries request-scoped values like cancellation signals, timeouts, and metadata
/// CRITICAL: Pass this as first parameter to ALL async trait methods
/// This enables proper cancellation and timeout handling
#[derive(Clone)]
pub struct Context {
    inner: Arc<ContextInner>,
}

struct ContextInner {
    deadline: Option<Instant>,
    values: RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>,
    done: watch::Receiver<bool>,
    _done_tx: watch::Sender<bool>,
}

impl Context {
    pub fn new() -> Self {
        let (done_tx, done_rx) = watch::channel(false);

        Self {
            inner: Arc::new(ContextInner {
                deadline: None,
                values: RwLock::new(HashMap::new()),
                done: done_rx,
                _done_tx: done_tx,
            }),
        }
    }

    pub fn with_timeout(self, timeout: Duration) -> Self {
        let deadline = Instant::now() + timeout;

        let (done_tx, done_rx) = watch::channel(false);

        let done_tx_clone = done_tx.clone();
        tokio::spawn(async move {
            time::sleep_until(deadline.into()).await;
            let _ = done_tx_clone.send(true);
        });

        Self {
            inner: Arc::new(ContextInner {
                deadline: Some(deadline),
                values: RwLock::new(HashMap::new()),
                done: done_rx,
                _done_tx: done_tx,
            }),
        }
    }

    pub async fn with_value<T: Send + Sync + 'static>(self, key: &str, value: T) -> Self {
        let mut values = self.inner.values.write().await;
        values.insert(key.to_string(), Box::new(value));
        drop(values);
        self
    }

    pub async fn get_value<T>(&self, key: &str) -> Option<T>
    where
        T: Send + Sync + Clone + 'static,
    {
        let values = self.inner.values.read().await;
        values.get(key).and_then(|v| v.downcast_ref::<T>()).cloned()
    }

    pub fn is_cancelled(&self) -> bool {
        *self.inner.done.borrow()
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.inner.deadline
    }

    /// Returns a channel that's closed when work done on behalf of this
    /// context should be cancelled
    pub fn done(&self) -> watch::Receiver<bool> {
        self.inner.done.clone()
    }

    pub fn cancel(&self) {
        let _ = self.inner._done_tx.send(true);
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn context_stores_and_retrieves_values() {
        let ctx = Context::new();
        let ctx = ctx.with_value("api_key", "secret123".to_string()).await;

        let value: Option<String> = ctx.get_value("api_key").await;
        assert_eq!(value, Some("secret123".to_string()));
    }

    #[tokio::test]
    async fn context_timeout_cancels() {
        let ctx = Context::new().with_timeout(Duration::from_millis(100));

        assert!(!ctx.is_cancelled());

        sleep(Duration::from_millis(150)).await;

        assert!(ctx.is_cancelled());
    }

    #[tokio::test]
    async fn context_manual_cancel() {
        let ctx = Context::new();

        assert!(!ctx.is_cancelled());

        ctx.cancel();

        assert!(ctx.is_cancelled());
    }

    #[tokio::test]
    async fn context_deadline() {
        let ctx = Context::new();
        assert!(ctx.deadline().is_none());

        let ctx_with_timeout = ctx.with_timeout(Duration::from_secs(1));
        assert!(ctx_with_timeout.deadline().is_some());
    }
}
