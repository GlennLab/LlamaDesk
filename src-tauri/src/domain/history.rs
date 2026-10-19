//! Value types persisted by the history store.
//!
//! Keeping these in the domain layer lets the storage port and its concrete
//! implementations share comparable types without depending on each other.

use serde::{Deserialize, Serialize};

/// A persisted server-metrics sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSample {
    /// Unix epoch milliseconds when the sample was captured.
    pub timestamp_ms: i64,
    /// Prompt processing throughput (tokens/sec).
    pub prompt_tokens_per_sec: Option<f64>,
    /// Generation throughput (tokens/sec).
    pub generation_tokens_per_sec: Option<f64>,
    /// Number of requests currently being processed.
    pub active_requests: Option<f64>,
    /// Number of deferred requests.
    pub deferred_requests: Option<f64>,
    /// Derived KV-cache usage ratio.
    pub kv_cache_usage_ratio: Option<f64>,
    /// Cumulative prompt tokens processed.
    pub prompt_tokens_total: Option<f64>,
    /// Cumulative predicted tokens.
    pub tokens_predicted_total: Option<f64>,
}

/// A persisted GPU sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSample {
    /// Unix epoch milliseconds when the sample was captured.
    pub timestamp_ms: i64,
    /// Zero-based GPU index the sample belongs to.
    pub gpu_index: u32,
    /// Utilization percentage, when reported.
    pub utilization_percent: Option<f64>,
    /// Used device memory in MiB, when reported.
    pub memory_used_mib: Option<u64>,
    /// Total device memory in MiB, when reported.
    pub memory_total_mib: Option<u64>,
    /// Core temperature in degrees Celsius, when reported.
    pub temperature_c: Option<f64>,
      /// Instantaneous power draw in Watts, when reported.
      pub power_draw_w: Option<f64>,
      /// Fan speed as percentage of maximum, when reported.
      pub fan_speed_percent: Option<f64>,
  }
