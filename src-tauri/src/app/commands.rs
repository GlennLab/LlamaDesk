//! Tauri commands exposed to the frontend.
//!
//! Every command returns a serializable value or a `String` error, as required
//! by Tauri's `invoke` bridge. Command names, argument names and return shapes
//! form a deliberately frozen contract the frontend mirrors: the function name
//! is the invoke name verbatim, and `rename_all = "camelCase"` maps snake_case
//! parameters to the camelCase keys the frontend sends.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::State;

use crate::app::monitor::{probe_once, retention_cutoff_ms};
use crate::app::state::AppState;
use crate::domain::history::{GpuSample, ServerSample};
use crate::domain::snapshot::{ConnectionState, MonitorSnapshot};
use crate::infra::config::{AppConfig, validate_server_url};
use crate::ports::storage::HistoryStore;

/// Upper bound for a single `test_connection` probe.
const TEST_TIMEOUT: Duration = Duration::from_secs(4);

/// Result of a one-shot connection test.
///
/// `null` is used for absent values (never coerced to `0`), matching the
/// contract's "missing values are null" rule.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    /// Whether the probe reached the server.
    pub ok: bool,
    /// Human-readable result description.
    pub message: String,
    /// Server role, when reported.
    pub role: Option<String>,
    /// Server build identifier, when reported.
    pub build_info: Option<String>,
    /// Number of models reported, when the probe succeeded.
    pub model_count: Option<u64>,
}

/// Returns the current snapshot.
#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> MonitorSnapshot {
    state.snapshot_clone()
}

/// Returns the current settings.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppConfig {
    state.current_config()
}

/// Validates, persists, and applies the supplied settings.
#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, config: AppConfig) -> Result<AppConfig, String> {
    state
        .apply_config(config)
        .map_err(|error| error.to_string())
}

/// Probes `url` (or the configured server) and reports the outcome.
///
/// The target is validated as an absolute `http`/`https` URL before a provider
/// is built, and the probe is bounded by [`TEST_TIMEOUT`] so the command can
/// never hang.
#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    url: Option<String>,
) -> Result<ConnectionTest, String> {
    let target = url.unwrap_or_else(|| state.current_config().server_url);
    validate_server_url(&target).map_err(|error| error.to_string())?;
    let provider = state
        .build_provider(&target)
        .map_err(|error| error.to_string())?;

    let probe = tokio::time::timeout(TEST_TIMEOUT, probe_once(provider.as_ref()))
        .await
        .map_err(|_| "connection test timed out".to_string())?;
    let (connection, props, model_count) = probe;

    let ok = connection == ConnectionState::Connected;
    let message = if ok {
        format!("Connected to {target}")
    } else if connection == ConnectionState::Disconnected {
        format!("Could not reach {target}")
    } else {
        format!("{target} responded with an error")
    };

    Ok(ConnectionTest {
        ok,
        message,
        role: props.as_ref().map(|props| props.role.clone()),
        build_info: props.as_ref().and_then(|props| props.build_info.clone()),
        // A successful probe with an unreadable `/models` is `None`, not zero.
        model_count: if ok {
            model_count.map(|count| count as u64)
        } else {
            None
        },
    })
}

/// Returns server samples with a timestamp in `[fromMs, toMs]`.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_server_history(
    state: State<'_, AppState>,
    from_ms: i64,
    to_ms: i64,
) -> Result<Vec<ServerSample>, String> {
    let store = store_of(state.inner())?;
    store
        .query_server_range(from_ms, to_ms)
        .await
        .map_err(|error| error.to_string())
}

/// Returns GPU samples with a timestamp in `[fromMs, toMs]`.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_gpu_history(
    state: State<'_, AppState>,
    from_ms: i64,
    to_ms: i64,
) -> Result<Vec<GpuSample>, String> {
    let store = store_of(state.inner())?;
    store
        .query_gpu_range(from_ms, to_ms)
        .await
        .map_err(|error| error.to_string())
}

/// Deletes samples older than the configured retention window and returns the
/// number of rows removed.
#[tauri::command]
pub async fn prune_history(state: State<'_, AppState>) -> Result<u64, String> {
    let store = store_of(state.inner())?;
    let cutoff = retention_cutoff_ms(
        state.current_config().retention_hours,
        chrono::Utc::now().timestamp_millis(),
    );
    store.prune(cutoff).await.map_err(|error| error.to_string())
}

/// Returns the history store, or a serializable error when history is disabled.
fn store_of(state: &AppState) -> Result<Arc<dyn HistoryStore>, String> {
    state
        .store()
        .ok_or_else(|| "history store is unavailable".to_string())
}
