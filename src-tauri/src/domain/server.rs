//! Types describing the inference server itself.

use serde::{Deserialize, Serialize};

/// Response body of `GET /health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerHealth {
    /// Server status, e.g. `"ok"`.
    pub status: String,
}

/// Response body of `GET /props`, normalized.
///
/// Recognized fields are typed; any additional fields the server reports are
/// preserved verbatim in [`ServerProps::extra`] so that nothing is lost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProps {
    /// Server role, e.g. `"router"` or `"model"`.
    pub role: String,
    /// Maximum number of managed instances (router only).
    pub max_instances: Option<u32>,
    /// Whether the router auto-loads models.
    pub models_autoload: Option<bool>,
    /// Default model alias, when configured.
    pub model_alias: Option<String>,
    /// Path of the underlying GGUF model, when reported.
    pub model_path: Option<String>,
    /// Build identifier string.
    pub build_info: Option<String>,
    /// Whether a CORS proxy is enabled.
    pub cors_proxy_enabled: Option<bool>,
    /// Any additional properties reported by the server.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
