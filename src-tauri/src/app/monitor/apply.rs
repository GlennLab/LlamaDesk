//! Merge slice results into the [`MonitorSnapshot`].
//!
//! These helpers are the per-slice application logic: the server, model and
//! hardware loops each hand their polled result to one of them, which folds it
//! into the shared snapshot and records or clears the corresponding slice error.

use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;

use crate::domain::error::MonitorError;
use crate::domain::hardware::GpuStats;
use crate::domain::history::{GpuSample, ServerSample};
use crate::domain::metrics::ServerMetrics;
use crate::domain::model::{ModelInfo, ModelStatus};
use crate::domain::server::{ServerHealth, ServerProps};
use crate::domain::slot::Slot;
use crate::domain::snapshot::{ConnectionState, MonitorSnapshot};

use super::{SLICE_HARDWARE, SLICE_METRICS, SLICE_SERVER, SLICE_SLOTS};

/// The three values produced by one successful server poll.
#[derive(Debug, Clone)]
pub struct ServerSlice {
    /// Response of `/health`.
    pub health: ServerHealth,
    /// Response of `/props`.
    pub props: ServerProps,
    /// Response of `/models`.
    pub models: Vec<ModelInfo>,
}

/// The slots and metrics fetched for a single model.
#[derive(Debug)]
pub struct ModelPoll {
    /// Result of `/slots?model=…`.
    pub slots: Result<Vec<Slot>, MonitorError>,
    /// Result of `/metrics?model=…`.
    pub metrics: Result<ServerMetrics, MonitorError>,
}

/// Classifies a provider failure into a connection state.
///
/// Only an actively refused connection means the server is unreachable
/// ([`ConnectionState::Disconnected`]); timeouts, DNS, and TLS failures mean it
/// did not answer cleanly, which is [`ConnectionState::Error`].
pub fn classify_connection_error(error: &MonitorError) -> ConnectionState {
    if error.is_connection_refused() {
        ConnectionState::Disconnected
    } else {
        ConnectionState::Error
    }
}

/// Returns the ids of models whose status is [`ModelStatus::Loaded`].
pub fn loaded_model_ids(models: &[ModelInfo]) -> Vec<String> {
    models
        .iter()
        .filter(|model| model.status == ModelStatus::Loaded)
        .map(|model| model.id.clone())
        .collect()
}

/// Applies a server-slice result.
///
/// On success the connection, health, props and models are replaced and the
/// server error cleared. On failure only the connection state and the error are
/// changed: previously-known `props`/`models` are preserved. Either way
/// `last_updated` is refreshed.
pub fn apply_server_result(
    snapshot: &mut MonitorSnapshot,
    result: Result<ServerSlice, MonitorError>,
) {
    match result {
        Ok(slice) => {
            snapshot.connection = ConnectionState::Connected;
            snapshot.health = Some(slice.health);
            snapshot.props = Some(slice.props);
            snapshot.loaded_models = loaded_model_ids(&slice.models);
            snapshot.models = slice.models;
            snapshot.clear_slice_error(SLICE_SERVER);
        }
        Err(error) => {
            snapshot.connection = classify_connection_error(&error);
            snapshot.set_slice_error(SLICE_SERVER, error.to_string());
        }
    }
    snapshot.last_updated = Some(Utc::now());
}

/// Merges the results for the currently loaded models into the `slots` and
/// `metrics` maps, dropping entries for models that are no longer loaded.
///
/// A per-model failure keeps that model's last-known value and records the
/// failure, so the UI does not lose live data because one model misbehaved.
/// Slice errors are only touched once at least one model was polled: a call
/// with no loaded models must not clear a previously recorded failure.
pub fn apply_model_results(
    snapshot: &mut MonitorSnapshot,
    loaded: &[String],
    results: &BTreeMap<String, ModelPoll>,
) {
    let loaded_set: BTreeSet<String> = loaded.iter().cloned().collect();
    let mut slots = snapshot.slots.clone();
    let mut metrics = snapshot.metrics.clone();
    let mut slot_errors = Vec::new();
    let mut metric_errors = Vec::new();
    let mut polled_any = false;

    for model in loaded {
        let Some(poll) = results.get(model) else {
            continue;
        };
        polled_any = true;
        match &poll.slots {
            Ok(list) => {
                slots.insert(model.clone(), list.clone());
            }
            Err(error) => slot_errors.push(format!("{model}: {error}")),
        }
        match &poll.metrics {
            Ok(value) => {
                metrics.insert(model.clone(), value.clone());
            }
            Err(error) => metric_errors.push(format!("{model}: {error}")),
        }
    }

    slots.retain(|model, _| loaded_set.contains(model));
    metrics.retain(|model, _| loaded_set.contains(model));
    snapshot.slots = slots;
    snapshot.metrics = metrics;

    if polled_any {
        set_or_clear_error(snapshot, SLICE_SLOTS, &slot_errors);
        set_or_clear_error(snapshot, SLICE_METRICS, &metric_errors);
    }
}

/// Applies a hardware-slice result.
///
/// `gpu_available` is always taken from the monitor so it stays honest even
/// when a snapshot attempt fails; on failure the previous `gpus` are retained.
pub fn apply_hardware_result(
    snapshot: &mut MonitorSnapshot,
    result: Result<Vec<GpuStats>, MonitorError>,
    available: bool,
) {
    match result {
        Ok(gpus) => {
            snapshot.gpus = gpus;
            snapshot.clear_slice_error(SLICE_HARDWARE);
        }
        Err(error) => snapshot.set_slice_error(SLICE_HARDWARE, error.to_string()),
    }
    snapshot.gpu_available = available;
}

/// Sets or clears a slice error depending on whether any failures occurred.
fn set_or_clear_error(snapshot: &mut MonitorSnapshot, slice: &str, errors: &[String]) {
    if errors.is_empty() {
        snapshot.clear_slice_error(slice);
    } else {
        snapshot.set_slice_error(slice, errors.join("; "));
    }
}

/// Builds a persisted server sample from `metrics`.
pub fn server_sample_from(metrics: &ServerMetrics, timestamp_ms: i64) -> ServerSample {
    ServerSample {
        timestamp_ms,
        prompt_tokens_per_sec: metrics.prompt_tokens_per_sec,
        generation_tokens_per_sec: metrics.generation_tokens_per_sec,
        active_requests: metrics.active_requests,
        deferred_requests: metrics.deferred_requests,
        kv_cache_usage_ratio: metrics.kv_cache_usage_ratio,
        prompt_tokens_total: metrics.prompt_tokens_total,
        tokens_predicted_total: metrics.tokens_predicted_total,
    }
}

/// Builds a persisted GPU sample from `gpu`.
pub fn gpu_sample_from(gpu: &GpuStats, timestamp_ms: i64) -> GpuSample {
    GpuSample {
        timestamp_ms,
        gpu_index: gpu.index,
        utilization_percent: gpu.utilization_percent,
        memory_used_mib: gpu.memory_used_mib,
        memory_total_mib: gpu.memory_total_mib,
          temperature_c: gpu.temperature_c,
          power_draw_w: gpu.power_draw_w,
          fan_speed_percent: gpu.fan_speed_percent,
      }
}
