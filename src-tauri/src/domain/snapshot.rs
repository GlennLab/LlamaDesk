//! Aggregated, point-in-time view of everything the UI needs.
//!
//! [`MonitorSnapshot`] is the single serializable payload the application hands
//! to the frontend. It is pure data: no I/O, no clock, no locks. Every field is
//! either a value captured by a monitor slice or `None` when that slice has not
//! produced data yet.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::hardware::GpuStats;
use crate::domain::metrics::ServerMetrics;
use crate::domain::model::ModelInfo;
use crate::domain::server::{ServerHealth, ServerProps};
use crate::domain::slot::Slot;

/// Default LlamaDesk server URL used when no configuration is present.
pub const DEFAULT_SERVER_URL: &str = "http://localhost:8080";

/// Connectivity state of the monitored server.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    /// No successful contact with the server.
    #[default]
    Disconnected,
    /// The server answered successfully.
    Connected,
    /// The server answered, but an error occurred.
    Error,
}

/// A per-slice failure captured while building the snapshot.
///
/// Slices never abort one another; a failure is recorded here instead and the
/// slice simply retains its previous value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceError {
    /// Name of the slice that failed (e.g. `"slots"`).
    pub slice: String,
    /// Human-readable failure description.
    pub message: String,
}

/// Aggregated, point-in-time view of everything the UI needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    /// Connectivity state of the server.
    pub connection: ConnectionState,
    /// Server health, when a `/health` response has been received.
    pub health: Option<ServerHealth>,
    /// Server properties, when a `/props` response has been received.
    pub props: Option<ServerProps>,
    /// Models known to the server.
    pub models: Vec<ModelInfo>,
    /// Identifiers of models currently loaded.
    pub loaded_models: Vec<String>,
    /// Decode slots keyed by model id.
    pub slots: BTreeMap<String, Vec<Slot>>,
    /// Server metrics keyed by model id.
    pub metrics: BTreeMap<String, ServerMetrics>,
    /// Point-in-time GPU statistics.
    pub gpus: Vec<GpuStats>,
    /// Whether GPU telemetry is available at all.
    pub gpu_available: bool,
    /// Base URL of the monitored server.
    pub server_url: String,
    /// When the snapshot was last refreshed.
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    /// Failures captured per slice during the last refresh.
    pub errors: Vec<SliceError>,
}

impl Default for MonitorSnapshot {
    fn default() -> Self {
        Self {
            connection: ConnectionState::Disconnected,
            health: None,
            props: None,
            models: Vec::new(),
            loaded_models: Vec::new(),
            slots: BTreeMap::new(),
            metrics: BTreeMap::new(),
            gpus: Vec::new(),
            gpu_available: false,
            server_url: DEFAULT_SERVER_URL.to_string(),
            last_updated: None,
            errors: Vec::new(),
        }
    }
}

impl MonitorSnapshot {
    /// Records a failure for `slice`, replacing any existing entry for it.
    ///
    /// Slice names are unique within `errors`, so repeated failures keep only
    /// the most recent message rather than accumulating duplicates.
    pub fn set_slice_error(&mut self, slice: &str, msg: String) {
        if let Some(existing) = self.errors.iter_mut().find(|e| e.slice == slice) {
            existing.message = msg;
        } else {
            self.errors.push(SliceError {
                slice: slice.to_string(),
                message: msg,
            });
        }
    }

    /// Removes the recorded failure for `slice`, if any.
    pub fn clear_slice_error(&mut self, slice: &str) {
        self.errors.retain(|e| e.slice != slice);
    }

    /// Returns `true` when the model `id` is currently loaded.
    pub fn is_loaded(&self, id: &str) -> bool {
        self.loaded_models.iter().any(|loaded| loaded == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_known_keys_in_camel_case() {
        let mut snapshot = MonitorSnapshot {
            gpu_available: true,
            last_updated: Some(chrono::Utc::now()),
            ..MonitorSnapshot::default()
        };
        snapshot.loaded_models.push("model-a".to_string());

        let value = serde_json::to_value(&snapshot).expect("serialise snapshot");
        let object = value.as_object().expect("snapshot is an object");

        assert!(object.contains_key("loadedModels"));
        assert!(object.contains_key("gpuAvailable"));
        assert!(object.contains_key("lastUpdated"));
        assert!(object.contains_key("serverUrl"));
    }

    #[test]
    fn connected_state_serialises_to_lowercase_string() {
        let value = serde_json::to_value(ConnectionState::Connected).expect("serialise state");
        assert_eq!(value, serde_json::json!("connected"));
    }

    #[test]
    fn slice_errors_replace_by_slice() {
        let mut snapshot = MonitorSnapshot::default();
        snapshot.set_slice_error("slots", "first".to_string());
        snapshot.set_slice_error("gpu", "hardware down".to_string());
        snapshot.set_slice_error("slots", "second".to_string());

        assert_eq!(snapshot.errors.len(), 2);
        let slots = snapshot
            .errors
            .iter()
            .find(|e| e.slice == "slots")
            .expect("slots error present");
        assert_eq!(slots.message, "second");

        snapshot.clear_slice_error("slots");
        assert_eq!(snapshot.errors.len(), 1);
        assert_eq!(snapshot.errors[0].slice, "gpu");
    }

    #[test]
    fn is_loaded_matches_exact_id() {
        let mut snapshot = MonitorSnapshot::default();
        snapshot
            .loaded_models
            .push("qwen-38-27b-mtp-coder".to_string());

        assert!(snapshot.is_loaded("qwen-38-27b-mtp-coder"));
        assert!(!snapshot.is_loaded("other"));
    }

    #[test]
    fn clear_slice_error_on_absent_slice_is_a_noop() {
        let mut snapshot = MonitorSnapshot::default();
        snapshot.set_slice_error("gpu", "down".to_string());

        snapshot.clear_slice_error("slots");

        assert_eq!(snapshot.errors.len(), 1);
        assert_eq!(snapshot.errors[0].slice, "gpu");
        assert_eq!(snapshot.errors[0].message, "down");
    }

    #[test]
    fn default_is_disconnected_and_empty() {
        let snapshot = MonitorSnapshot::default();
        assert_eq!(snapshot.connection, ConnectionState::Disconnected);
        assert!(!snapshot.gpu_available);
        assert_eq!(snapshot.server_url, DEFAULT_SERVER_URL);
        assert!(snapshot.errors.is_empty());
    }
}
