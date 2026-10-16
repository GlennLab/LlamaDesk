//! Types describing GPU hardware.

use serde::{Deserialize, Serialize};

/// A point-in-time snapshot of a single GPU.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuStats {
    /// Zero-based GPU index reported by the driver.
    pub index: u32,
    /// Marketing name of the GPU.
    pub name: String,
    /// Utilization percentage, when reported.
    pub utilization_percent: Option<f64>,
    /// Total device memory in MiB, when reported.
    pub memory_total_mib: Option<u64>,
    /// Used device memory in MiB, when reported.
    pub memory_used_mib: Option<u64>,
    /// Core temperature in degrees Celsius, when reported.
    pub temperature_c: Option<f64>,
    /// Instantaneous power draw in Watts, when reported.
    pub power_draw_w: Option<f64>,
    /// Enforced power limit in Watts, when reported.
    pub power_limit_w: Option<f64>,
}
