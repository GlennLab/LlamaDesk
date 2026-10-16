//! The four independent polling loops and their slice-local storage helpers.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::watch;

use crate::app::state::{AppState, ProviderFactory};
use crate::domain::error::MonitorError;
use crate::domain::snapshot::MonitorSnapshot;
use crate::infra::config::AppConfig;
use crate::ports::hardware::HardwareMonitor;
use crate::ports::provider::InferenceProvider;
use crate::ports::storage::HistoryStore;

use super::{
    ModelPoll, SLICE_STORAGE, ServerSlice, SharedProvider, apply_hardware_result,
    apply_model_results, apply_server_result, gpu_sample_from, retention_cutoff_ms,
    server_sample_from,
};

/// Minimum time between history prune passes.
const PRUNE_INTERVAL: Duration = Duration::from_secs(3_600);

/// Server slice: `health` → `props` → `models`.
pub(super) async fn server_loop(
    state: AppState,
    provider: SharedProvider,
    factory: ProviderFactory,
    mut config_rx: watch::Receiver<AppConfig>,
    mut url: String,
) {
    loop {
        let config = config_rx.borrow_and_update().clone();

        if config.server_url != url {
            match factory(&config.server_url) {
                Ok(next) => {
                    *provider.write().await = next;
                    log::info!("monitor: switched server url to {}", config.server_url);
                    url = config.server_url.clone();
                }
                Err(error) => {
                    log::warn!(
                        "monitor: cannot use server url {}: {error}",
                        config.server_url
                    );
                }
            }
        }

        let current = provider.read().await.clone();
        let result = poll_server(current.as_ref()).await;
        let snapshot_url = url.clone();
        state.update_snapshot(|snapshot| {
            snapshot.server_url = snapshot_url;
            apply_server_result(snapshot, result);
        });
        state.emit_snapshot();

        tokio::time::sleep(Duration::from_millis(config.server_interval_ms)).await;
    }
}

/// Per-model slice: `slots` and `metrics` for every loaded model.
pub(super) async fn model_loop(
    state: AppState,
    provider: SharedProvider,
    mut config_rx: watch::Receiver<AppConfig>,
) {
    loop {
        let config = config_rx.borrow_and_update().clone();
        let loaded = state.snapshot_clone().loaded_models;
        let current = provider.read().await.clone();

        let mut results = BTreeMap::new();
        for model in &loaded {
            let slots = current.slots(model).await;
            let metrics = current.metrics(model).await;
            results.insert(model.clone(), ModelPoll { slots, metrics });
        }

        state.update_snapshot(|snapshot| apply_model_results(snapshot, &loaded, &results));
        state.emit_snapshot();

        tokio::time::sleep(Duration::from_millis(config.slots_interval_ms)).await;
    }
}

/// Hardware slice: GPU telemetry.
pub(super) async fn hardware_loop(
    state: AppState,
    hardware: Arc<dyn HardwareMonitor>,
    mut config_rx: watch::Receiver<AppConfig>,
) {
    loop {
        let config = config_rx.borrow_and_update().clone();
        let result = hardware.snapshot().await;
        let available = hardware.available();
        state.update_snapshot(|snapshot| apply_hardware_result(snapshot, result, available));
        state.emit_snapshot();

        tokio::time::sleep(Duration::from_millis(config.hardware_interval_ms)).await;
    }
}

/// Storage slice: persist samples and periodically prune old rows.
pub(super) async fn storage_loop(state: AppState, mut config_rx: watch::Receiver<AppConfig>) {
    let mut last_prune: Option<Instant> = None;
    loop {
        let config = config_rx.borrow_and_update().clone();

        if let Some(store) = state.store() {
            let mut errors = persist_once(
                store.as_ref(),
                &state.snapshot_clone(),
                Utc::now().timestamp_millis(),
            )
            .await;

            if last_prune.is_none_or(|at| at.elapsed() >= PRUNE_INTERVAL) {
                last_prune = Some(Instant::now());
                errors.extend(prune_once(store.as_ref(), &config).await);
            }

            if errors.is_empty() {
                state.update_snapshot(|snapshot| snapshot.clear_slice_error(SLICE_STORAGE));
            } else {
                let message = errors.join("; ");
                log::warn!("monitor: storage: {message}");
                state.update_snapshot(|snapshot| snapshot.set_slice_error(SLICE_STORAGE, message));
            }
            state.emit_snapshot();
        }

        tokio::time::sleep(Duration::from_millis(config.storage_interval_ms)).await;
    }
}

/// Persists one server sample (from the first loaded model with metrics) and one
/// GPU sample per GPU, collecting failures rather than aborting on the first.
async fn persist_once(
    store: &dyn HistoryStore,
    snapshot: &MonitorSnapshot,
    now_ms: i64,
) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(metrics) = snapshot
        .loaded_models
        .iter()
        .find_map(|model| snapshot.metrics.get(model))
    {
        let sample = server_sample_from(metrics, now_ms);
        if let Err(error) = store.insert_server_sample(&sample).await {
            errors.push(format!("server sample: {error}"));
        }
    }

    for gpu in &snapshot.gpus {
        let sample = gpu_sample_from(gpu, now_ms);
        if let Err(error) = store.insert_gpu_sample(&sample).await {
            errors.push(format!("gpu {}: {error}", gpu.index));
        }
    }

    errors
}

/// Prunes rows older than the configured retention window, returning any error.
async fn prune_once(store: &dyn HistoryStore, config: &AppConfig) -> Option<String> {
    let cutoff = retention_cutoff_ms(config.retention_hours, Utc::now().timestamp_millis());
    match store.prune(cutoff).await {
        Ok(removed) => {
            log::debug!("monitor: pruned {removed} history row(s)");
            None
        }
        Err(error) => Some(format!("prune: {error}")),
    }
}

/// Polls the three server-slice endpoints as a single all-or-nothing unit.
async fn poll_server(provider: &dyn InferenceProvider) -> Result<ServerSlice, MonitorError> {
    let health = provider.health().await?;
    let props = provider.props().await?;
    let models = provider.models().await?;
    Ok(ServerSlice {
        health,
        props,
        models,
    })
}
