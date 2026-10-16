//! Port for reading GPU hardware statistics.

use async_trait::async_trait;

use crate::domain::error::MonitorError;
use crate::domain::hardware::GpuStats;

/// Source of GPU telemetry.
#[async_trait]
pub trait HardwareMonitor: Send + Sync {
    /// Captures the current statistics for every visible GPU.
    ///
    /// When no supported hardware is present this yields an empty vector rather
    /// than an error.
    async fn snapshot(&self) -> Result<Vec<GpuStats>, MonitorError>;

    /// Whether a supported backend is available on this machine.
    ///
    /// Implementations may probe once and cache the result.
    fn available(&self) -> bool;
}
