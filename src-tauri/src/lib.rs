pub mod app;
pub mod domain;
pub mod infra;
pub mod ports;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use crate::app::monitor::MonitorService;
use crate::app::state::{AppState, ProviderFactory};
use crate::domain::error::MonitorError;
use crate::domain::snapshot::DEFAULT_SERVER_URL;
use crate::infra::config::AppConfig;
use crate::infra::db::SqliteStore;
use crate::infra::gpu::NvidiaSmiMonitor;
use crate::infra::llama::LlamaRouterClient;
use crate::ports::hardware::HardwareMonitor;
use crate::ports::provider::InferenceProvider;
use crate::ports::storage::HistoryStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let handle = app.handle().clone();
            let state = init_state(&handle)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::get_snapshot,
            app::commands::get_settings,
            app::commands::update_settings,
            app::commands::test_connection,
            app::commands::get_server_history,
            app::commands::get_gpu_history,
            app::commands::prune_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}

/// Builds application state, opens the history database, and starts the monitor
/// loops.
///
/// A database failure is logged and monitoring continues with history disabled,
/// so a broken database can never stop live monitoring.
fn init_state(handle: &tauri::AppHandle) -> Result<AppState, Box<dyn std::error::Error>> {
    let config_dir = prepare_dir(handle.path().app_config_dir(), "config")?;
    let config_path = config_dir.join("config.json");

    let data_dir = prepare_dir(handle.path().app_data_dir(), "data")?;
    let db_path = data_dir.join("llamadesk.db");

    let mut config = AppConfig::load(&config_path);
    if config.validate().is_err() {
        log::warn!(
            "config: invalid serverUrl {:?}; falling back to {DEFAULT_SERVER_URL}",
            config.server_url
        );
        config.server_url = DEFAULT_SERVER_URL.to_string();
        config.validate()?;
    }

    let factory: ProviderFactory = build_provider;
    let provider = build_provider_with_fallback(factory, &config.server_url)?;
    let hardware: Arc<dyn HardwareMonitor> = Arc::new(NvidiaSmiMonitor::new());

    let store = match tauri::async_runtime::block_on(SqliteStore::connect(&db_path)) {
        Ok(store) => Some(Arc::new(store) as Arc<dyn HistoryStore>),
        Err(error) => {
            log::warn!("storage: history disabled ({error})");
            None
        }
    };

    let state = AppState::new(config, config_path, store, handle.clone(), factory);
    let service = MonitorService::new(state.clone(), provider, hardware);
    tauri::async_runtime::spawn(async move {
        service.spawn_loops();
    });

    Ok(state)
}

/// Ensures `dir` exists and returns it, surfacing resolution or creation errors
/// to the caller.
fn prepare_dir(
    dir: Result<PathBuf, tauri::Error>,
    label: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = dir?;
    std::fs::create_dir_all(&dir)?;
    log::debug!("{label} directory: {}", dir.display());
    Ok(dir)
}

/// Builds a llama.cpp router client for `url` as an object-safe provider.
///
/// This is the concrete factory wired into [`AppState`] and the monitor's
/// server loop; keeping it here means the transport type never leaks into the
/// app layer, which only sees [`ProviderFactory`].
fn build_provider(url: &str) -> Result<Arc<dyn InferenceProvider>, MonitorError> {
    let client = LlamaRouterClient::builder(url)?.build()?;
    Ok(Arc::new(client))
}

/// Builds a provider for `url`, falling back to the default URL on failure.
fn build_provider_with_fallback(
    factory: ProviderFactory,
    url: &str,
) -> Result<Arc<dyn InferenceProvider>, Box<dyn std::error::Error>> {
    match factory(url) {
        Ok(provider) => Ok(provider),
        Err(error) => {
            log::warn!(
                "provider: cannot use {url} ({error}); falling back to {DEFAULT_SERVER_URL}"
            );
            Ok(factory(DEFAULT_SERVER_URL)?)
        }
    }
}
