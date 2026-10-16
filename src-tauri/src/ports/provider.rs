//! Port for reading state from an inference server.

use async_trait::async_trait;

use crate::domain::error::MonitorError;
use crate::domain::metrics::ServerMetrics;
use crate::domain::model::ModelInfo;
use crate::domain::server::{ServerHealth, ServerProps};
use crate::domain::slot::Slot;

/// Read-only view of an inference server (e.g. a llama.cpp router).
#[async_trait]
pub trait InferenceProvider: Send + Sync {
    /// Returns the server's health status.
    async fn health(&self) -> Result<ServerHealth, MonitorError>;

    /// Returns the server's properties.
    async fn props(&self) -> Result<ServerProps, MonitorError>;

    /// Returns the models known to the server.
    async fn models(&self) -> Result<Vec<ModelInfo>, MonitorError>;

    /// Returns the decode slots for `model`.
    ///
    /// Servers that report no slots (e.g. an unloaded model) yield an empty
    /// vector rather than an error.
    async fn slots(&self, model: &str) -> Result<Vec<Slot>, MonitorError>;

    /// Returns the server metrics for `model`.
    async fn metrics(&self, model: &str) -> Result<ServerMetrics, MonitorError>;

    /// Human-readable name for this provider, used in logs and diagnostics.
    fn display_name(&self) -> &str;
}
