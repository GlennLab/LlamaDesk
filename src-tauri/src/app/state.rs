//! Shared application state handed to every Tauri command and monitor loop.
//!
//! [`AppState`] is deliberately cheap to clone: every field is either an `Arc`,
//! a `watch` sender, or another small handle, so it can be cloned into each
//! background loop and re-created by Tauri for every command invocation.
//!
//! The snapshot is guarded by a [`std::sync::Mutex`] rather than a `tokio`
//! lock. The critical section is a short, purely in-memory mutation that never
//! spans an `.await`, so an async lock would only add runtime overhead and an
//! unnecessary suspension point.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

use crate::domain::error::MonitorError;
use crate::domain::snapshot::MonitorSnapshot;
use crate::infra::config::AppConfig;
use crate::ports::provider::InferenceProvider;
use crate::ports::storage::HistoryStore;

/// Name of the Tauri event carrying a full [`MonitorSnapshot`] payload.
pub const EVENT_SNAPSHOT: &str = "monitor:snapshot";

/// Builds an [`InferenceProvider`] for a given server URL.
///
/// A plain function pointer keeps the concrete transport (the llama.cpp client)
/// out of [`AppState`] and the monitor loops: production wires in the real
/// client once in `lib.rs`, while tests inject a fake. It is `Copy`, so it can
/// be captured by every loop without cloning.
pub type ProviderFactory = fn(&str) -> Result<Arc<dyn InferenceProvider>, MonitorError>;

/// Cheap-to-clone handle to the application's live state.
#[derive(Clone)]
pub struct AppState {
    snapshot: Arc<Mutex<MonitorSnapshot>>,
    config_tx: watch::Sender<AppConfig>,
    config_path: PathBuf,
    store: Option<Arc<dyn HistoryStore>>,
    provider_factory: ProviderFactory,
    // `None` only in headless tests, where there is no running app to emit to.
    app_handle: Option<AppHandle>,
    /// Snapshot last pushed to the frontend, used to suppress unchanged emits.
    /// `None` means nothing has been emitted yet, so the first emit always fires.
    last_emitted: Arc<Mutex<Option<MonitorSnapshot>>>,
}

impl AppState {
    /// Creates the shared state for a running application.
    pub fn new(
        config: AppConfig,
        config_path: PathBuf,
        store: Option<Arc<dyn HistoryStore>>,
        app_handle: AppHandle,
        provider_factory: ProviderFactory,
    ) -> Self {
        Self::build(
            config,
            config_path,
            store,
            provider_factory,
            Some(app_handle),
        )
    }

    /// Creates the shared state for a headless test (no [`AppHandle`]).
    ///
    /// The default factory always errors; tests that exercise provider building
    /// install their own concrete factory through `build`.
    #[cfg(test)]
    pub(crate) fn for_test(config_path: PathBuf, store: Option<Arc<dyn HistoryStore>>) -> Self {
        fn unavailable(_url: &str) -> Result<Arc<dyn InferenceProvider>, MonitorError> {
            Err(MonitorError::Config(
                "no provider factory configured for tests".to_string(),
            ))
        }
        Self::build(AppConfig::default(), config_path, store, unavailable, None)
    }

    fn build(
        config: AppConfig,
        config_path: PathBuf,
        store: Option<Arc<dyn HistoryStore>>,
        provider_factory: ProviderFactory,
        app_handle: Option<AppHandle>,
    ) -> Self {
        let snapshot = MonitorSnapshot {
            server_url: config.server_url.clone(),
            ..MonitorSnapshot::default()
        };
        let (config_tx, _config_rx) = watch::channel(config);
        Self {
            snapshot: Arc::new(Mutex::new(snapshot)),
            config_tx,
            config_path,
            store,
            provider_factory,
            app_handle,
            last_emitted: Arc::new(Mutex::new(None)),
        }
    }

    fn lock_snapshot(&self) -> MutexGuard<'_, MonitorSnapshot> {
        // A poisoned lock only means another thread panicked mid-mutation; the
        // data itself is still consistent, so we recover rather than propagate.
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Returns the current settings.
    pub fn current_config(&self) -> AppConfig {
        self.config_tx.borrow().clone()
    }

    /// Subscribes to configuration changes.
    ///
    /// Each monitor loop owns its own receiver so that a settings update wakes
    /// every loop on its next tick.
    pub fn subscribe(&self) -> watch::Receiver<AppConfig> {
        self.config_tx.subscribe()
    }

    /// Returns a clone of the current snapshot.
    pub fn snapshot_clone(&self) -> MonitorSnapshot {
        self.lock_snapshot().clone()
    }

    /// Applies `update` to the snapshot in place under the lock.
    pub fn update_snapshot<F>(&self, update: F)
    where
        F: FnOnce(&mut MonitorSnapshot),
    {
        update(&mut self.lock_snapshot());
    }

    /// Returns a handle to the history store, when one is available.
    pub fn store(&self) -> Option<Arc<dyn HistoryStore>> {
        self.store.clone()
    }

    /// Returns the factory used to build providers, for injection into loops.
    pub fn provider_factory(&self) -> ProviderFactory {
        self.provider_factory
    }

    /// Builds an [`InferenceProvider`] for `url` using the configured factory.
    pub fn build_provider(&self, url: &str) -> Result<Arc<dyn InferenceProvider>, MonitorError> {
        (self.provider_factory)(url)
    }

    /// Emits the current snapshot on [`EVENT_SNAPSHOT`], logging any failure.
    ///
    /// The emit is suppressed when the snapshot is materially identical to the
    /// last one sent, so a steady-state poll does not push an unchanged payload
    /// to the frontend every tick. A missing listener is not an error, so this
    /// never fails the caller.
    pub fn emit_snapshot(&self) {
        let Some(app_handle) = &self.app_handle else {
            return;
        };
        let snapshot = self.snapshot_clone();
        {
            let mut last = self
                .last_emitted
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if !materially_changed(last.as_ref(), &snapshot) {
                return;
            }
            *last = Some(snapshot.clone());
        }
        if let Err(error) = app_handle.emit(EVENT_SNAPSHOT, &snapshot) {
            log::warn!("failed to emit {EVENT_SNAPSHOT}: {error}");
        }
    }

    /// Validates, persists, and broadcasts `next`, returning the validated form.
    ///
    /// Validation runs first, so an invalid configuration is neither saved nor
    /// applied. The broadcast uses `send_replace`, which succeeds even before
    /// any loop has subscribed.
    pub fn apply_config(&self, mut next: AppConfig) -> Result<AppConfig, MonitorError> {
        next.validate()?;
        next.save(&self.config_path)?;
        self.config_tx.send_replace(next.clone());
        Ok(next)
    }
}

/// Returns `true` when the meaningful contents of `after` differ from `before`.
///
/// `last_updated` advances on every poll and must not, by itself, trigger a
/// frontend emit — otherwise the UI receives an identical payload every tick.
/// An absent `before` always counts as a change so the first snapshot is sent.
fn materially_changed(before: Option<&MonitorSnapshot>, after: &MonitorSnapshot) -> bool {
    let Some(before) = before else {
        return true;
    };
    let mut normalized = after.clone();
    normalized.last_updated = before.last_updated;
    &normalized != before
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::infra::config::MIN_SERVER_INTERVAL_MS;

    /// Returns a unique path inside the system temp directory whose parent does
    /// not exist yet.
    fn temp_config_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = format!(
            "llamadesk-state-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        std::env::temp_dir().join(unique).join("config.json")
    }

    #[test]
    fn apply_config_rejects_invalid_url_without_persisting() {
        let path = temp_config_path();
        let state = AppState::for_test(path.clone(), None);
        let invalid = AppConfig {
            server_url: "not a url".to_string(),
            ..AppConfig::default()
        };

        assert!(state.apply_config(invalid).is_err());
        assert!(!path.exists(), "an invalid config must not be written");
    }

    #[test]
    fn apply_config_persists_valid_config_and_updates_current() {
        let path = temp_config_path();
        let state = AppState::for_test(path.clone(), None);
        let valid = AppConfig {
            server_url: "http://example.com:1234".to_string(),
            server_interval_ms: 1,
            ..AppConfig::default()
        };

        let applied = state.apply_config(valid).expect("valid config");

        // Validation clamps the below-floor interval.
        assert_eq!(applied.server_interval_ms, MIN_SERVER_INTERVAL_MS);
        assert_eq!(state.current_config().server_url, "http://example.com:1234");
        assert_eq!(AppConfig::load(&path).server_url, "http://example.com:1234");
    }

    #[test]
    fn materially_changed_ignores_timestamp_only_differences() {
        let base = MonitorSnapshot::default();
        let mut later = base.clone();
        later.last_updated = Some(chrono::Utc::now());

        assert!(!materially_changed(Some(&base), &later));
        assert!(materially_changed(None, &later));
    }

    #[test]
    fn materially_changed_detects_real_content_changes() {
        let base = MonitorSnapshot::default();
        let mut changed = base.clone();
        changed.server_url = "http://other.example:9".to_string();

        assert!(materially_changed(Some(&base), &changed));
    }
}
