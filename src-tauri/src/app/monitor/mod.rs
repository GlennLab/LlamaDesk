//! Background polling loops that keep the [`MonitorSnapshot`] fresh.
//!
//! Four independent tokio tasks share the [`AppState`]:
//!
//! 1. **server** — `health` → `props` → `models` every `serverIntervalMs`.
//! 2. **model** — `slots` and `metrics` for each loaded model every
//!    `slotsIntervalMs`.
//! 3. **hardware** — GPU telemetry every `hardwareIntervalMs`.
//! 4. **storage** — persists samples and prunes old rows every
//!    `storageIntervalMs`, with a prune pass at most once per hour.
//!
//! A failure in one slice is recorded in `errors[]` and never aborts the other
//! slices; a storage failure never stops live monitoring. Each loop reads the
//! *current* intervals from the configuration watch channel every tick, so a
//! settings change applies within one interval.

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::app::state::AppState;
use crate::ports::hardware::HardwareMonitor;
use crate::ports::provider::InferenceProvider;

mod apply;
mod loops;
mod probe;

#[cfg(test)]
mod tests;

pub use apply::{
    ModelPoll, ServerSlice, apply_hardware_result, apply_model_results, apply_server_result,
    classify_connection_error, gpu_sample_from, loaded_model_ids, server_sample_from,
};
pub use probe::{probe_once, retention_cutoff_ms};

use loops::{hardware_loop, model_loop, server_loop, storage_loop};

/// Slice name recorded for server-slice failures.
pub const SLICE_SERVER: &str = "server";
/// Slice name recorded for slots failures.
pub const SLICE_SLOTS: &str = "slots";
/// Slice name recorded for metrics failures.
pub const SLICE_METRICS: &str = "metrics";
/// Slice name recorded for hardware failures.
pub const SLICE_HARDWARE: &str = "hardware";
/// Slice name recorded for storage failures.
pub const SLICE_STORAGE: &str = "storage";

/// Provider shared between the server and model loops, so a change of
/// `serverUrl` is observed by both without restarting the loops.
type SharedProvider = Arc<RwLock<Arc<dyn InferenceProvider>>>;

/// Owns the polling loops and starts them in [`MonitorService::spawn_loops`].
pub struct MonitorService {
    state: AppState,
    provider: Arc<dyn InferenceProvider>,
    hardware: Arc<dyn HardwareMonitor>,
}

impl MonitorService {
    /// Creates the service; call [`MonitorService::spawn_loops`] to start it.
    pub fn new(
        state: AppState,
        provider: Arc<dyn InferenceProvider>,
        hardware: Arc<dyn HardwareMonitor>,
    ) -> Self {
        Self {
            state,
            provider,
            hardware,
        }
    }

    /// Spawns the four loops, consuming the service.
    ///
    /// Must be called from within a tokio runtime context (for example inside
    /// [`tauri::async_runtime`]). The tasks run for the lifetime of the process
    /// and detach when this returns, so no join handles are retained.
    pub fn spawn_loops(self) {
        let state = self.state.clone();
        let provider: SharedProvider = Arc::new(RwLock::new(self.provider.clone()));
        let hardware = self.hardware.clone();
        let initial_url = state.current_config().server_url;

        tokio::spawn(server_loop(
            state.clone(),
            provider.clone(),
            state.provider_factory(),
            state.subscribe(),
            initial_url,
        ));
        tokio::spawn(model_loop(state.clone(), provider, state.subscribe()));
        tokio::spawn(hardware_loop(state.clone(), hardware, state.subscribe()));
        let storage_rx = state.subscribe();
        tokio::spawn(storage_loop(state, storage_rx));
    }
}
