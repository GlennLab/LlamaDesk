//! Application configuration: defaults, loading, saving, and validation.
//!
//! Configuration is stored as pretty-printed JSON. Loading is deliberately
//! forgiving — a missing, unreadable, or corrupt file yields
//! [`AppConfig::default`] rather than an error, so the app always starts.

use std::collections::HashSet;
use std::path::Path;

use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::domain::error::MonitorError;

pub use crate::domain::snapshot::DEFAULT_SERVER_URL;

/// Default polling interval for the server slice, in milliseconds.
pub const DEFAULT_SERVER_INTERVAL_MS: u64 = 2000;
/// Default polling interval for the slots slice, in milliseconds.
pub const DEFAULT_SLOTS_INTERVAL_MS: u64 = 1000;
/// Default polling interval for the hardware slice, in milliseconds.
pub const DEFAULT_HARDWARE_INTERVAL_MS: u64 = 2000;
/// Default polling interval for the storage slice, in milliseconds.
pub const DEFAULT_STORAGE_INTERVAL_MS: u64 = 5000;
/// Default history retention window, in hours.
pub const DEFAULT_RETENTION_HOURS: u64 = 24;

/// Minimum polling interval for the server slice, in milliseconds.
pub const MIN_SERVER_INTERVAL_MS: u64 = 250;
/// Minimum polling interval for the slots slice, in milliseconds.
pub const MIN_SLOTS_INTERVAL_MS: u64 = 250;
/// Minimum polling interval for the hardware slice, in milliseconds.
pub const MIN_HARDWARE_INTERVAL_MS: u64 = 250;
/// Minimum polling interval for the storage slice, in milliseconds.
pub const MIN_STORAGE_INTERVAL_MS: u64 = 1000;
/// Minimum retention window, in hours.
pub const MIN_RETENTION_HOURS: u64 = 1;
/// Maximum retention window, in hours (one year). Caps how far back the prune
/// pass can reach, so a mistyped value cannot retain history indefinitely.
pub const MAX_RETENTION_HOURS: u64 = 24 * 365;

/// A named server profile: a label plus the llama.cpp router URL it points to.
///
/// `AppConfig` keeps `server_url` as the effective URL of the active profile;
/// profiles are the named source of truth for each endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    /// Human-readable profile name.
    pub name: String,
    /// Base URL of the llama.cpp router for this profile.
    pub server_url: String,
}

/// Default for [`AppConfig::profiles`] when a stored config omits the field:
/// empty, so [`AppConfig::load`] can migrate the stored `serverUrl` into a
/// `"Default"` profile.
fn default_profiles() -> Vec<ServerProfile> {
    Vec::new()
}

/// User-configurable application settings.
///
/// Deserialization is tolerant of partial files: any missing field falls back to
/// its default via the container-level `#[serde(default)]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    /// Base URL of the llama.cpp router to monitor (effective URL of the active profile).
    pub server_url: String,
    /// Named server profiles; the active profile's URL is mirrored in `server_url`.
    #[serde(default = "default_profiles")]
    pub profiles: Vec<ServerProfile>,
    /// Name of the currently active profile.
    pub active_profile: String,
    /// Polling interval for the server slice, in milliseconds.
    pub server_interval_ms: u64,
    /// Polling interval for the slots slice, in milliseconds.
    pub slots_interval_ms: u64,
    /// Polling interval for the hardware slice, in milliseconds.
    pub hardware_interval_ms: u64,
    /// Polling interval for the storage slice, in milliseconds.
    pub storage_interval_ms: u64,
    /// History retention window, in hours.
    pub retention_hours: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            profiles: vec![ServerProfile {
                name: "Default".to_string(),
                server_url: DEFAULT_SERVER_URL.to_string(),
            }],
            active_profile: "Default".to_string(),
            server_interval_ms: DEFAULT_SERVER_INTERVAL_MS,
            slots_interval_ms: DEFAULT_SLOTS_INTERVAL_MS,
            hardware_interval_ms: DEFAULT_HARDWARE_INTERVAL_MS,
            storage_interval_ms: DEFAULT_STORAGE_INTERVAL_MS,
            retention_hours: DEFAULT_RETENTION_HOURS,
        }
    }
}

impl AppConfig {
    /// Loads configuration from `path`.
    ///
    /// Returns [`AppConfig::default`] and logs a warning if the file is missing,
    /// unreadable, or not valid JSON. This function never panics.
    pub fn load(path: &Path) -> AppConfig {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                log::warn!(
                    "config: failed to read {} ({error}); using defaults",
                    path.display()
                );
                return AppConfig::default();
            }
        };

        match serde_json::from_str::<AppConfig>(&contents) {
            Ok(mut config) => {
                // Migrate pre-profiles config files: synthesize a "Default" profile
                // from the stored serverUrl so old config.json keeps working.
                if config.profiles.is_empty() {
                    config.profiles.push(ServerProfile {
                        name: "Default".to_string(),
                        server_url: config.server_url.clone(),
                    });
                    if config.active_profile.is_empty() {
                        config.active_profile = "Default".to_string();
                    }
                }
                config
            }
            Err(error) => {
                log::warn!(
                    "config: failed to parse {} ({error}); using defaults",
                    path.display()
                );
                AppConfig::default()
            }
        }
    }

    /// Writes the configuration to `path` as pretty-printed JSON.
    ///
    /// Creates any missing parent directories.
    pub fn save(&self, path: &Path) -> Result<(), MonitorError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| {
                MonitorError::Config(format!(
                    "failed to create config directory {}: {error}",
                    parent.display()
                ))
            })?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|error| {
            MonitorError::Config(format!("failed to serialise config: {error}"))
        })?;

        std::fs::write(path, json).map_err(|error| {
            MonitorError::Config(format!(
                "failed to write config {}: {error}",
                path.display()
            ))
        })
    }

    /// Normalises the configuration in place, clamping values to safe floors
    /// and ceiling.
    ///
    /// Validates every profile URL (see [`validate_server_url`]), enforces
    /// unique non-empty profile names, requires at least one profile, falls
    /// back to the first profile when `active_profile` names nothing, and
    /// normalises `server_url` to the active profile's URL. Clamps
    /// `retention_hours` into `[MIN_RETENTION_HOURS, MAX_RETENTION_HOURS]`.
    pub fn validate(&mut self) -> Result<(), MonitorError> {
        // Every profile URL must be an absolute http/https URL.
        for profile in &self.profiles {
            validate_server_url(&profile.server_url)?;
        }

        // Profile names must be unique and non-empty.
        let mut seen_names = HashSet::new();
        for profile in &self.profiles {
            if profile.name.is_empty() {
                return Err(MonitorError::Config(
                    "profile names must not be empty".to_string(),
                ));
            }
            if !seen_names.insert(profile.name.clone()) {
                return Err(MonitorError::Config(format!(
                    "duplicate profile name `{}`",
                    profile.name
                )));
            }
        }

        // At least one profile is required.
        if self.profiles.is_empty() {
            return Err(MonitorError::Config(
                "at least one server profile is required".to_string(),
            ));
        }

        // The active profile must name an existing profile; fall back to the
        // first profile and re-normalise server_url when it does not.
        if !self
            .profiles
            .iter()
            .any(|profile| profile.name == self.active_profile)
        {
            self.active_profile = self.profiles[0].name.clone();
        }

        // Normalise server_url to the active profile's URL.
        let active_url = self
            .profiles
            .iter()
            .find(|profile| profile.name == self.active_profile)
            .map(|profile| profile.server_url.clone())
            .expect("active profile is guaranteed to exist after the fallback above");
        self.server_url = active_url;

        self.server_interval_ms = self.server_interval_ms.max(MIN_SERVER_INTERVAL_MS);
        self.slots_interval_ms = self.slots_interval_ms.max(MIN_SLOTS_INTERVAL_MS);
        self.hardware_interval_ms = self.hardware_interval_ms.max(MIN_HARDWARE_INTERVAL_MS);
        self.storage_interval_ms = self.storage_interval_ms.max(MIN_STORAGE_INTERVAL_MS);
        self.retention_hours = self
            .retention_hours
            .clamp(MIN_RETENTION_HOURS, MAX_RETENTION_HOURS);

        Ok(())
    }
}

/// Validates that `url` is an absolute `http`/`https` URL.
///
/// The scheme is required explicitly, so a bare `host:port` is rejected. Shared
/// by [`AppConfig::validate`] and the `test_connection` command, so both enforce
/// exactly the same rule at their boundary.
pub fn validate_server_url(url: &str) -> Result<(), MonitorError> {
    let parsed = Url::parse(url).map_err(|error| {
        MonitorError::Config(format!(
            "serverUrl must be an absolute URL with an http or https scheme: {error}"
        ))
    })?;

    match parsed.scheme() {
        "http" | "https" => Ok(()),
        other => Err(MonitorError::Config(format!(
            "serverUrl scheme must be http or https, got `{other}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    /// Returns a unique path inside the system temp directory whose parent does
    /// not exist yet.
    fn temp_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = format!(
            "llamadesk-config-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        std::env::temp_dir().join(unique).join("config.json")
    }

    #[test]
    fn default_round_trips_through_json() {
        let original = AppConfig::default();
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: AppConfig = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }

    #[test]
    fn invalid_profile_url_is_rejected() {
        let mut config = AppConfig {
            profiles: vec![ServerProfile {
                name: "Default".to_string(),
                server_url: "localhost:8080".to_string(),
            }],
            active_profile: "Default".to_string(),
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());

        let mut config = AppConfig {
            profiles: vec![ServerProfile {
                name: "Default".to_string(),
                server_url: "ftp://example.com".to_string(),
            }],
            active_profile: "Default".to_string(),
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn valid_profile_url_passes_with_explicit_scheme() {
        let mut config = AppConfig {
            profiles: vec![ServerProfile {
                name: "Default".to_string(),
                server_url: "https://example.com:9090".to_string(),
            }],
            active_profile: "Default".to_string(),
            ..AppConfig::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn default_has_one_default_profile() {
        let config = AppConfig::default();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, "Default");
        assert_eq!(config.profiles[0].server_url, DEFAULT_SERVER_URL);
        assert_eq!(config.active_profile, "Default");
        assert_eq!(config.server_url, DEFAULT_SERVER_URL);
    }

    #[test]
    fn validate_rejects_duplicate_profile_names() {
        let mut config = AppConfig {
            profiles: vec![
                ServerProfile {
                    name: "Default".to_string(),
                    server_url: "http://localhost:8080".to_string(),
                },
                ServerProfile {
                    name: "Default".to_string(),
                    server_url: "http://localhost:9090".to_string(),
                },
            ],
            active_profile: "Default".to_string(),
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_profile_name() {
        let mut config = AppConfig {
            profiles: vec![ServerProfile {
                name: "".to_string(),
                server_url: "http://localhost:8080".to_string(),
            }],
            active_profile: "".to_string(),
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_profiles() {
        let mut config = AppConfig {
            profiles: vec![],
            active_profile: "Default".to_string(),
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_falls_back_to_first_profile_and_normalizes_url() {
        let mut config = AppConfig {
            server_url: "http://example.com:1234".to_string(),
            profiles: vec![
                ServerProfile {
                    name: "First".to_string(),
                    server_url: "http://localhost:8080".to_string(),
                },
                ServerProfile {
                    name: "Second".to_string(),
                    server_url: "http://localhost:9090".to_string(),
                },
            ],
            active_profile: "Nonexistent".to_string(),
            ..AppConfig::default()
        };
        config.validate().expect("valid config");

        assert_eq!(config.active_profile, "First");
        assert_eq!(config.server_url, "http://localhost:8080");
    }

    #[test]
    fn validate_normalizes_server_url_to_active_profile() {
        let mut config = AppConfig {
            server_url: "http://example.com:1234".to_string(),
            profiles: vec![ServerProfile {
                name: "Default".to_string(),
                server_url: "http://localhost:9090".to_string(),
            }],
            active_profile: "Default".to_string(),
            ..AppConfig::default()
        };
        config.validate().expect("valid config");

        assert_eq!(config.server_url, "http://localhost:9090");
    }

    #[test]
    fn load_migrates_empty_profiles_from_server_url() {
        let path = temp_path();
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        std::fs::write(&path, r#"{"serverUrl":"http://example.com:1234"}"#).expect("write");

        let config = AppConfig::load(&path);
        assert_eq!(config.server_url, "http://example.com:1234");
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, "Default");
        assert_eq!(config.profiles[0].server_url, "http://example.com:1234");
        assert_eq!(config.active_profile, "Default");
    }

    #[test]
    fn load_keeps_existing_profiles() {
        let path = temp_path();
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        std::fs::write(
            &path,
            r#"{"serverUrl":"http://example.com:1234","profiles":[{"name":"Default","serverUrl":"http://localhost:8080"}],"activeProfile":"Default"}"#,
        )
        .expect("write");

        let config = AppConfig::load(&path);
        assert_eq!(config.server_url, "http://example.com:1234");
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, "Default");
        assert_eq!(config.active_profile, "Default");
    }

    #[test]
    fn validate_server_url_accepts_http_and_https() {
        assert!(validate_server_url("http://localhost:8080").is_ok());
        assert!(validate_server_url("https://example.com:9090").is_ok());
    }

    #[test]
    fn validate_server_url_rejects_bare_host_and_other_schemes() {
        assert!(validate_server_url("localhost:8080").is_err());
        assert!(validate_server_url("ftp://example.com").is_err());
        assert!(validate_server_url("not a url").is_err());
    }

    #[test]
    fn validation_clamps_retention_into_range() {
        let mut config = AppConfig {
            retention_hours: 0,
            ..AppConfig::default()
        };
        config.validate().expect("valid config");
        assert_eq!(config.retention_hours, MIN_RETENTION_HOURS);

        let mut config = AppConfig {
            retention_hours: u64::MAX,
            ..AppConfig::default()
        };
        config.validate().expect("valid config");
        assert_eq!(config.retention_hours, MAX_RETENTION_HOURS);
    }

    #[test]
    fn validation_clamps_below_floor_values() {
        let mut config = AppConfig {
            server_interval_ms: 10,
            slots_interval_ms: 10,
            hardware_interval_ms: 10,
            storage_interval_ms: 10,
            retention_hours: 0,
            ..AppConfig::default()
        };
        config.validate().expect("valid config");

        assert_eq!(config.server_interval_ms, MIN_SERVER_INTERVAL_MS);
        assert_eq!(config.slots_interval_ms, MIN_SLOTS_INTERVAL_MS);
        assert_eq!(config.hardware_interval_ms, MIN_HARDWARE_INTERVAL_MS);
        assert_eq!(config.storage_interval_ms, MIN_STORAGE_INTERVAL_MS);
        assert_eq!(config.retention_hours, MIN_RETENTION_HOURS);
    }

    #[test]
    fn corrupt_json_falls_back_to_defaults() {
        let path = temp_path();
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        std::fs::write(&path, "{ not valid json").expect("write");

        assert_eq!(AppConfig::load(&path), AppConfig::default());
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let path = temp_path();
        assert_eq!(AppConfig::load(&path), AppConfig::default());
    }

    #[test]
    fn partial_json_fills_missing_fields_with_defaults() {
        let path = temp_path();
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        std::fs::write(&path, r#"{"serverUrl":"http://example.com:1234"}"#).expect("write");

        let config = AppConfig::load(&path);
        assert_eq!(config.server_url, "http://example.com:1234");
        assert_eq!(config.server_interval_ms, DEFAULT_SERVER_INTERVAL_MS);
        assert_eq!(config.retention_hours, DEFAULT_RETENTION_HOURS);
    }

    #[test]
    fn save_creates_parent_directories_and_round_trips() {
        let path = temp_path();
        let config = AppConfig {
            server_url: "http://example.com:1234".to_string(),
            ..AppConfig::default()
        };
        config.save(&path).expect("save");
        assert_eq!(AppConfig::load(&path), config);
    }
}
