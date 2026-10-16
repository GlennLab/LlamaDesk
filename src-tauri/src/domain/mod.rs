//! Pure domain types for LlamaDesk.
//!
//! This module contains no I/O and depends only on `serde`/`serde_json`. Every
//! type serializes with `camelCase` field names so the TypeScript frontend can
//! consume them directly, and missing values are represented as [`Option::None`]
//! (serialized as `null`) rather than being coerced to zero.

pub mod error;
pub mod hardware;
pub mod history;
pub mod metrics;
pub mod model;
pub mod server;
pub mod slot;
pub mod snapshot;

pub use error::{BoxError, MonitorError};
pub use hardware::GpuStats;
pub use history::{GpuSample, ServerSample};
pub use metrics::{LabelSet, PrometheusMetrics, ServerMetrics};
pub use model::{ModelArchitecture, ModelInfo, ModelStatus};
pub use server::{ServerHealth, ServerProps};
pub use slot::{NextToken, Slot};
pub use snapshot::{ConnectionState, DEFAULT_SERVER_URL, MonitorSnapshot, SliceError};
