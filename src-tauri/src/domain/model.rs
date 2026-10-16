//! Types describing a model known to the router.

use serde::{Deserialize, Serialize};

/// Load state reported for a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelStatus {
    /// Model is loaded and ready to serve.
    Loaded,
    /// Model is known but not currently loaded.
    Unloaded,
    /// Model is currently loading.
    Loading,
    /// Any other status string reported by the server.
    Unknown(String),
}

impl ModelStatus {
    /// Returns the canonical lowercase status string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Loaded => "loaded",
            Self::Unloaded => "unloaded",
            Self::Loading => "loading",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl From<&str> for ModelStatus {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "loaded" => Self::Loaded,
            "unloaded" => Self::Unloaded,
            "loading" => Self::Loading,
            _ => Self::Unknown(value.to_string()),
        }
    }
}

impl From<String> for ModelStatus {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Input/output modalities declared by a model's architecture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelArchitecture {
    /// Modalities accepted as input (e.g. `"text"`, `"image"`).
    pub input_modalities: Vec<String>,
    /// Modalities produced as output.
    pub output_modalities: Vec<String>,
}

/// A single entry from `GET /models`, normalized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Unique model identifier.
    pub id: String,
    /// Alternative identifiers the model answers to.
    pub aliases: Vec<String>,
    /// Free-form tags attached to the model.
    pub tags: Vec<String>,
    /// Current load state.
    pub status: ModelStatus,
    /// Command-line arguments the server reported for the loaded instance.
    pub status_args: Vec<String>,
    /// Where the model came from, when reported.
    pub source: Option<String>,
    /// Whether the model may be removed.
    pub can_remove: bool,
    /// Context window length, when reported.
    pub context_length: Option<u64>,
    /// Architecture description, when reported.
    pub architecture: Option<ModelArchitecture>,
}
