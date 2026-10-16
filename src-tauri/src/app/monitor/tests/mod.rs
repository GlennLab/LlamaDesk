//! Unit tests for the monitor module, split by the code they cover.

mod apply;
mod probe;

use crate::domain::model::{ModelInfo, ModelStatus};
use crate::domain::server::{ServerHealth, ServerProps};
use crate::domain::slot::Slot;

fn model(id: &str, status: ModelStatus) -> ModelInfo {
    ModelInfo {
        id: id.to_string(),
        aliases: Vec::new(),
        tags: Vec::new(),
        status,
        status_args: Vec::new(),
        source: None,
        can_remove: false,
        context_length: None,
        architecture: None,
    }
}

fn slot(id: i32) -> Slot {
    Slot {
        id,
        n_ctx: None,
        speculative: None,
        is_processing: false,
        id_task: None,
        n_prompt_tokens: None,
        n_prompt_tokens_processed: None,
        n_prompt_tokens_cache: None,
        next_token: Vec::new(),
        params: None,
    }
}

fn props() -> ServerProps {
    ServerProps {
        role: "router".to_string(),
        max_instances: Some(1),
        models_autoload: Some(true),
        model_alias: None,
        model_path: None,
        build_info: Some("b2794".to_string()),
        cors_proxy_enabled: Some(false),
        extra: serde_json::Map::new(),
    }
}

fn health() -> ServerHealth {
    ServerHealth {
        status: "ok".to_string(),
    }
}
