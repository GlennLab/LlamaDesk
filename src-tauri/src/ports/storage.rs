//! Port for persisting and querying metric history.

use async_trait::async_trait;

use crate::domain::error::MonitorError;
use crate::domain::history::{GpuSample, ServerSample};

/// Append-only store of metric samples with range queries and retention.
#[async_trait]
pub trait HistoryStore: Send + Sync {
    /// Persists a single server-metrics sample.
    async fn insert_server_sample(&self, sample: &ServerSample) -> Result<(), MonitorError>;

    /// Persists a single GPU sample.
    async fn insert_gpu_sample(&self, sample: &GpuSample) -> Result<(), MonitorError>;

    /// Returns server samples with a timestamp in `[from_ms, to_ms]`, oldest first.
    async fn query_server_range(
        &self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<ServerSample>, MonitorError>;

    /// Returns GPU samples with a timestamp in `[from_ms, to_ms]`, oldest first.
    async fn query_gpu_range(
        &self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<GpuSample>, MonitorError>;

    /// Deletes samples older than `retention_ms` and returns the number removed.
    async fn prune(&self, retention_ms: i64) -> Result<u64, MonitorError>;
}
