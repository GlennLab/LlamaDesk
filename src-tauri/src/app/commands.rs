//! Tauri commands exposed to the frontend.
//!
//! Every command returns a serializable value or a `String` error, as required
//! by Tauri's `invoke` bridge. Command names, argument names and return shapes
//! form a deliberately frozen contract the frontend mirrors: the function name
//! is the invoke name verbatim, and `rename_all = "camelCase"` maps snake_case
//! parameters to the camelCase keys the frontend sends.

use std::path::{Path, PathBuf};
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

/// Result of running the llama.cpp update script.
///
/// `old_version`/`new_version` are `null` when the version could not be read
/// (missing binary, empty `--version` output, etc.), never coerced to a
/// placeholder string.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    /// Whether the update script exited successfully.
    pub success: bool,
    /// Version reported by `build/bin/llama-cli` (or `main`) before the update.
    pub old_version: Option<String>,
    /// Version reported by the same binary after the update.
    pub new_version: Option<String>,
    /// Combined stdout/stderr of the update script, trimmed.
    pub output: String,
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

/// Runs the llama.cpp update script and reports the old vs new version.
///
/// Resolves `LLAMA_DIR` (default `$HOME/llama.cpp`) and the script path
/// (`scripts/update-llama.sh`, overridable via `LLAMA_UPDATE_SCRIPT`), then
/// delegates to [`update_llama_inner`] on a blocking thread so the command
/// stays non-blocking.
#[allow(unused_variables)] // `state` is reserved for future use; the command is self-contained.
#[tauri::command]
pub async fn update_llama(state: State<'_, AppState>) -> Result<UpdateResult, String> {
    let llama_dir = std::env::var("LLAMA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::home_dir().unwrap_or_default().join("llama.cpp"));
    let script = std::env::var("LLAMA_UPDATE_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("scripts/update-llama.sh")
        });
    tauri::async_runtime::spawn_blocking(move || update_llama_inner(&llama_dir, &script))
        .await
        .map_err(|error| error.to_string())?
}

/// Synchronously runs the update script and reads versions before and after.
///
/// Testable directly without Tauri. Never panics: missing binaries or scripts
/// surface as `Result` errors rather than crashes.
fn update_llama_inner(llama_dir: &Path, script: &Path) -> Result<UpdateResult, String> {
    if !script.exists() {
        return Err(format!("update script not found: {}", script.display()));
    }

    let binary = llama_dir.join("build/bin/llama-cli");
    let fallback = llama_dir.join("build/bin/main");

    let old_version = read_version(&binary, &fallback).ok();

    let output = std::process::Command::new("bash")
        .arg(script)
        .output()
        .map_err(|error| format!("failed to run update script: {error}"))?;
    let success = output.status.success();
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
    .trim()
    .to_string();

    let new_version = read_version(&binary, &fallback).ok();

    Ok(UpdateResult {
        success,
        old_version,
        new_version,
        output,
    })
}

/// Returns the first non-empty trimmed line of `binary --version`.
///
/// Tries `primary` then `fallback`, skipping binaries that do not exist.
/// Returns an error when neither binary produces a version line.
fn read_version(primary: &Path, fallback: &Path) -> Result<String, String> {
    let mut spawn_error: Option<String> = None;
    for binary in [primary, fallback] {
        if !binary.exists() {
            continue;
        }
        let output = match std::process::Command::new(binary).arg("--version").output() {
            Ok(output) => output,
            Err(error) => {
                spawn_error = Some(format!("failed to run {}: {error}", binary.display()));
                continue;
            }
        };
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let line = line.trim();
            if !line.is_empty() {
                return Ok(line.to_string());
            }
        }
    }
    Err(match spawn_error {
        Some(error) => {
            format!(
                "no version output from {} or {}: {error}",
                primary.display(),
                fallback.display()
            )
        }
        None => format!(
            "no version output from {} or {}",
            primary.display(),
            fallback.display()
        ),
    })
}

/// Returns the history store, or a serializable error when history is disabled.
fn store_of(state: &AppState) -> Result<Arc<dyn HistoryStore>, String> {
    state
        .store()
        .ok_or_else(|| "history store is unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Creates a unique directory under the system temp dir.
    fn unique_temp_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "llamadesk-update-test-{}-{}",
            std::process::id(),
            n
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// Writes `content` to `path` and marks it executable (Unix).
    fn make_script(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create script parent dir");
        }
        let mut file = fs::File::create(path).expect("create script file");
        writeln!(file, "{content}").expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = file.metadata().expect("script metadata").permissions();
            permissions.set_mode(0o755);
            file.set_permissions(permissions)
                .expect("set script executable");
        }
    }

    #[test]
    fn read_version_returns_first_non_empty_line() {
        let dir = unique_temp_dir();
        let bin = dir.join("build/bin/llama-cli");
        make_script(
            &bin,
            "#!/usr/bin/env bash\necho ''\necho 'llama.cpp b1234'\necho 'extra'\n",
        );
        let fallback = dir.join("build/bin/main");

        let version = read_version(&bin, &fallback).expect("version from primary");

        assert_eq!(version, "llama.cpp b1234");
    }

    #[test]
    fn read_version_falls_back_to_secondary_binary() {
        let dir = unique_temp_dir();
        let primary = dir.join("build/bin/llama-cli");
        let fallback = dir.join("build/bin/main");
        make_script(
            &fallback,
            "#!/usr/bin/env bash\necho 'llama.cpp fallback-9'\n",
        );

        let version = read_version(&primary, &fallback).expect("version from fallback");

        assert_eq!(version, "llama.cpp fallback-9");
    }

    #[test]
    fn read_version_errors_when_no_binary_exists() {
        let dir = unique_temp_dir();
        let primary = dir.join("build/bin/llama-cli");
        let fallback = dir.join("build/bin/main");

        let error = read_version(&primary, &fallback).unwrap_err();

        assert!(error.contains("no version output"));
    }

    #[test]
    fn update_llama_inner_missing_script_errors() {
        let dir = unique_temp_dir();
        let script = dir.join("scripts/update-llama.sh");

        let error = update_llama_inner(&dir, &script).unwrap_err();

        assert!(error.contains("update script not found"));
    }

    #[test]
    fn update_llama_inner_reports_old_and_new_version() {
        let dir = unique_temp_dir();

        // Fake binary reporting the old version.
        let bin = dir.join("build/bin/llama-cli");
        make_script(&bin, "#!/usr/bin/env bash\necho 'llama.cpp 1.0.0'\n");

        // Fake upgrade script that replaces the binary with a new version.
        let script = dir.join("scripts/update-llama.sh");
        make_script(
            &script,
            r#"#!/usr/bin/env bash
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
LLAMA_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
BIN="$LLAMA_DIR/build/bin/llama-cli"
cat > "$BIN" <<'INNER'
#!/usr/bin/env bash
echo "llama.cpp 2.0.0"
INNER
chmod +x "$BIN"
echo "llama.cpp 2.0.0"
"#,
        );

        let result = update_llama_inner(&dir, &script).expect("update should run");

        assert!(result.success, "update should succeed: {}", result.output);
        assert_eq!(result.old_version.as_deref(), Some("llama.cpp 1.0.0"));
        assert_eq!(result.new_version.as_deref(), Some("llama.cpp 2.0.0"));
        assert!(result.output.contains("2.0.0"));
    }
}
