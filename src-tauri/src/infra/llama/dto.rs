//! Lenient deserialization structs mirroring the raw llama.cpp router JSON.
//!
//! Every field is `Option` or `#[serde(default)]` so that missing or unknown
//! fields never fail a parse. Each DTO converts into a normalized domain type.

use serde::Deserialize;

use crate::domain::model::{ModelArchitecture, ModelInfo, ModelStatus};
use crate::domain::server::{ServerHealth, ServerProps};
use crate::domain::slot::{NextToken, Slot};

/// `GET /health` response.
#[derive(Debug, Default, Deserialize)]
pub struct HealthDto {
    /// Server status string.
    #[serde(default)]
    pub status: String,
}

impl From<HealthDto> for ServerHealth {
    fn from(dto: HealthDto) -> Self {
        Self { status: dto.status }
    }
}

/// `GET /props` response. Unrecognized keys are captured by `extra`.
#[derive(Debug, Default, Deserialize)]
pub struct PropsDto {
    /// Server role.
    #[serde(default)]
    pub role: String,
    /// Maximum number of managed instances.
    #[serde(default)]
    pub max_instances: Option<u32>,
    /// Whether models are auto-loaded.
    #[serde(default)]
    pub models_autoload: Option<bool>,
    /// Default model alias.
    #[serde(default)]
    pub model_alias: Option<String>,
    /// Underlying model path.
    #[serde(default)]
    pub model_path: Option<String>,
    /// Build identifier.
    #[serde(default)]
    pub build_info: Option<String>,
    /// Whether a CORS proxy is enabled.
    #[serde(default)]
    pub cors_proxy_enabled: Option<bool>,
    /// Any additional properties reported by the server.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl From<PropsDto> for ServerProps {
    fn from(dto: PropsDto) -> Self {
        Self {
            role: dto.role,
            max_instances: dto.max_instances,
            models_autoload: dto.models_autoload,
            model_alias: dto.model_alias,
            model_path: dto.model_path,
            build_info: dto.build_info,
            cors_proxy_enabled: dto.cors_proxy_enabled,
            extra: dto.extra,
        }
    }
}

/// `GET /models` (or `/v1/models`) response envelope.
#[derive(Debug, Default, Deserialize)]
pub struct ModelsResponseDto {
    /// The model entries.
    #[serde(default)]
    pub data: Vec<ModelDto>,
}

/// A single model entry.
#[derive(Debug, Default, Deserialize)]
pub struct ModelDto {
    /// Model identifier.
    #[serde(default)]
    pub id: String,
    /// Alternative identifiers.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Free-form tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Load-state block.
    #[serde(default)]
    pub status: Option<ModelStatusDto>,
    /// Provenance string.
    #[serde(default)]
    pub source: Option<String>,
    /// Whether the model may be removed.
    #[serde(default)]
    pub can_remove: bool,
    /// Context window length.
    #[serde(default)]
    pub context_length: Option<u64>,
    /// Architecture description.
    #[serde(default)]
    pub architecture: Option<ArchitectureDto>,
}

/// The `status` block of a model entry.
#[derive(Debug, Default, Deserialize)]
pub struct ModelStatusDto {
    /// Status value, e.g. `"loaded"`.
    #[serde(default)]
    pub value: Option<String>,
    /// Command-line arguments for the loaded instance.
    #[serde(default)]
    pub args: Vec<String>,
    /// Preset name, when reported.
    #[serde(default)]
    pub preset: Option<String>,
}

/// The `architecture` block of a model entry.
#[derive(Debug, Default, Deserialize)]
pub struct ArchitectureDto {
    /// Accepted input modalities.
    #[serde(default)]
    pub input_modalities: Vec<String>,
    /// Produced output modalities.
    #[serde(default)]
    pub output_modalities: Vec<String>,
}

impl From<ArchitectureDto> for ModelArchitecture {
    fn from(dto: ArchitectureDto) -> Self {
        Self {
            input_modalities: dto.input_modalities,
            output_modalities: dto.output_modalities,
        }
    }
}

impl From<ModelDto> for ModelInfo {
    fn from(dto: ModelDto) -> Self {
        let (status, status_args) = match dto.status {
            Some(status) => {
                let value = status
                    .value
                    .as_deref()
                    .map(ModelStatus::from)
                    .unwrap_or(ModelStatus::Unloaded);
                (value, status.args)
            }
            None => (ModelStatus::Unloaded, Vec::new()),
        };
        Self {
            id: dto.id,
            aliases: dto.aliases,
            tags: dto.tags,
            status,
            status_args,
            source: dto.source,
            can_remove: dto.can_remove,
            context_length: dto.context_length,
            architecture: dto.architecture.map(ModelArchitecture::from),
        }
    }
}

/// A single entry from `GET /slots?model=<id>`.
#[derive(Debug, Default, Deserialize)]
pub struct SlotDto {
    /// Slot index.
    #[serde(default)]
    pub id: i32,
    /// Configured context size.
    #[serde(default)]
    pub n_ctx: Option<u64>,
    /// Whether speculative decoding is enabled.
    #[serde(default)]
    pub speculative: Option<bool>,
    /// Whether the slot is processing a request.
    #[serde(default)]
    pub is_processing: bool,
    /// Identifier of the assigned task.
    #[serde(default)]
    pub id_task: Option<u64>,
    /// Total prompt tokens.
    #[serde(default)]
    pub n_prompt_tokens: Option<u64>,
    /// Processed prompt tokens.
    #[serde(default)]
    pub n_prompt_tokens_processed: Option<u64>,
    /// Cached prompt tokens.
    #[serde(default)]
    pub n_prompt_tokens_cache: Option<u64>,
    /// Next-token statistics.
    #[serde(default)]
    pub next_token: Vec<NextTokenDto>,
    /// Opaque generation parameters.
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// Next-token statistics within a slot entry.
#[derive(Debug, Default, Deserialize)]
pub struct NextTokenDto {
    /// Whether another token can be produced.
    #[serde(default)]
    pub has_next_token: Option<bool>,
    /// Whether the next token begins a new line.
    #[serde(default)]
    pub has_new_line: Option<bool>,
    /// Tokens remaining to decode. A negative value (llama.cpp uses `-1`) means
    /// "no limit / unlimited" and is emitted for idle slots.
    #[serde(default)]
    pub n_remain: Option<i64>,
    /// Tokens decoded so far.
    #[serde(default)]
    pub n_decoded: Option<u64>,
}

impl From<NextTokenDto> for NextToken {
    fn from(dto: NextTokenDto) -> Self {
        Self {
            has_next_token: dto.has_next_token,
            has_new_line: dto.has_new_line,
            n_remain: dto.n_remain,
            n_decoded: dto.n_decoded,
        }
    }
}

impl From<SlotDto> for Slot {
    fn from(dto: SlotDto) -> Self {
        Self {
            id: dto.id,
            n_ctx: dto.n_ctx,
            speculative: dto.speculative,
            is_processing: dto.is_processing,
            id_task: dto.id_task,
            n_prompt_tokens: dto.n_prompt_tokens,
            n_prompt_tokens_processed: dto.n_prompt_tokens_processed,
            n_prompt_tokens_cache: dto.n_prompt_tokens_cache,
            next_token: dto.next_token.into_iter().map(NextToken::from).collect(),
            params: dto.params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_parses() {
        let dto: HealthDto = serde_json::from_str(r#"{"status":"ok"}"#).expect("health json");
        let health: ServerHealth = dto.into();
        assert_eq!(health.status, "ok");
    }

    #[test]
    fn health_defaults_when_field_missing() {
        let dto: HealthDto = serde_json::from_str("{}").expect("empty health json");
        let health: ServerHealth = dto.into();
        assert_eq!(health.status, "");
    }

    #[test]
    fn props_keeps_unknown_fields_in_extra() {
        let json = r#"{
            "role": "router",
            "max_instances": 1,
            "models_autoload": true,
            "model_alias": null,
            "build_info": "b2794-62df1815",
            "cors_proxy_enabled": false,
            "default_generation_settings": {"temperature": 0.8},
            "ui_settings": {"theme": "dark"},
            "unknown_thing": 7
        }"#;
        let dto: PropsDto = serde_json::from_str(json).expect("props json");
        let props: ServerProps = dto.into();
        assert_eq!(props.role, "router");
        assert_eq!(props.max_instances, Some(1));
        assert_eq!(props.models_autoload, Some(true));
        assert_eq!(props.model_alias, None);
        assert_eq!(props.build_info.as_deref(), Some("b2794-62df1815"));
        assert_eq!(props.cors_proxy_enabled, Some(false));
        assert!(props.extra.contains_key("default_generation_settings"));
        assert!(props.extra.contains_key("ui_settings"));
        assert!(props.extra.contains_key("unknown_thing"));
    }

    #[test]
    fn models_parse_mixed_statuses_and_missing_context_length() {
        let json = r#"{
            "data": [
                {
                    "id": "qwen2.5-7b",
                    "aliases": ["default"],
                    "tags": ["chat"],
                    "status": {"value": "loaded", "args": ["--ctx-size", "8192"], "preset": "default"},
                    "architecture": {"input_modalities": ["text"], "output_modalities": ["text"]},
                    "source": "hf",
                    "can_remove": true,
                    "context_length": 32768
                },
                {
                    "id": "llama-3-8b",
                    "aliases": [],
                    "tags": [],
                    "status": {"value": "unloaded", "args": []},
                    "can_remove": false
                },
                {
                    "id": "vision-model",
                    "status": {"value": "loading", "args": []},
                    "architecture": {"input_modalities": ["text", "image"], "output_modalities": ["text"]},
                    "can_remove": true
                },
                {
                    "id": "mystery-model",
                    "status": {"value": "something-new", "args": []},
                    "can_remove": false
                }
            ]
        }"#;
        let dto: ModelsResponseDto = serde_json::from_str(json).expect("models json");
        let models: Vec<ModelInfo> = dto.data.into_iter().map(ModelInfo::from).collect();

        assert_eq!(models.len(), 4);
        assert_eq!(models[0].status, ModelStatus::Loaded);
        assert_eq!(models[0].context_length, Some(32768));
        assert_eq!(models[0].status_args, vec!["--ctx-size", "8192"]);
        assert_eq!(
            models[0]
                .architecture
                .as_ref()
                .map(|a| a.input_modalities.clone()),
            Some(vec!["text".to_string()])
        );

        assert_eq!(models[1].status, ModelStatus::Unloaded);
        assert_eq!(models[1].context_length, None);

        assert_eq!(models[2].status, ModelStatus::Loading);
        assert_eq!(models[2].context_length, None);

        assert_eq!(
            models[3].status,
            ModelStatus::Unknown("something-new".to_string())
        );
    }

    #[test]
    fn slots_parse_full_object() {
        let json = r#"[{
            "id": 0,
            "n_ctx": 4096,
            "speculative": false,
            "is_processing": true,
            "id_task": 42,
            "n_prompt_tokens": 100,
            "n_prompt_tokens_processed": 100,
            "n_prompt_tokens_cache": 7,
            "params": {"temperature": 0.8},
            "next_token": [{"has_next_token": true, "has_new_line": false, "n_remain": 128, "n_decoded": 12}]
        }]"#;
        let dtos: Vec<SlotDto> = serde_json::from_str(json).expect("slots json");
        let slots: Vec<Slot> = dtos.into_iter().map(Slot::from).collect();

        assert_eq!(slots.len(), 1);
        let slot = &slots[0];
        assert_eq!(slot.id, 0);
        assert_eq!(slot.n_ctx, Some(4096));
        assert_eq!(slot.speculative, Some(false));
        assert!(slot.is_processing);
        assert_eq!(slot.id_task, Some(42));
        assert_eq!(slot.n_prompt_tokens_cache, Some(7));
        assert_eq!(slot.next_token.len(), 1);
        assert_eq!(slot.next_token[0].n_decoded, Some(12));
        assert!(slot.params.is_some());
    }

    #[test]
    fn slots_real_qwen_mtp_coder_payload_parses() {
        // Verbatim captured response from GET /slots?model=qwen-38-27b-mtp-coder.
        // Idle slots (1 and 2) report `n_remain: -1` (unlimited); the busy slot 0
        // reports a positive `n_remain`. Before `n_remain` became signed, the
        // negative idle values failed the whole array parse.
        let json = include_str!("../../../tests/fixtures/slots_qwen_mtp_coder.json");
        let dtos: Vec<SlotDto> = serde_json::from_str(json).expect("real slots json");
        let slots: Vec<Slot> = dtos.into_iter().map(Slot::from).collect();

        assert_eq!(slots.len(), 3);

        assert_eq!(slots[0].id, 0);
        assert!(slots[0].is_processing);
        assert_eq!(slots[0].next_token[0].n_remain, Some(23051));

        assert_eq!(slots[1].id, 1);
        assert_eq!(slots[2].id, 2);
        for slot in &slots[1..] {
            assert!(!slot.is_processing);
            assert_eq!(slot.next_token[0].n_remain, Some(-1));
        }

        for slot in &slots {
            assert_eq!(slot.n_ctx, Some(200000));
        }
    }

    #[test]
    fn slot_with_missing_fields_survives() {
        let dtos: Vec<SlotDto> = serde_json::from_str(r#"[{"id": 3, "is_processing": false}]"#)
            .expect("minimal slot json");
        let slots: Vec<Slot> = dtos.into_iter().map(Slot::from).collect();
        assert_eq!(slots[0].id, 3);
        assert!(!slots[0].is_processing);
        assert_eq!(slots[0].n_ctx, None);
        assert!(slots[0].next_token.is_empty());
        assert_eq!(slots[0].params, None);
    }

    #[test]
    fn next_token_without_fields_is_all_none() {
        let dtos: Vec<SlotDto> =
            serde_json::from_str(r#"[{"id": 0, "is_processing": true, "next_token": [{}]}]"#)
                .expect("slot with empty next_token json");
        let slots: Vec<Slot> = dtos.into_iter().map(Slot::from).collect();
        assert_eq!(slots[0].next_token.len(), 1);
        let token = &slots[0].next_token[0];
        assert_eq!(token.has_next_token, None);
        assert_eq!(token.has_new_line, None);
        assert_eq!(token.n_remain, None);
        assert_eq!(token.n_decoded, None);
    }

    #[test]
    fn model_status_round_trips_through_contract_json() {
        assert_eq!(
            serde_json::to_value(ModelStatus::Loaded).expect("serialise loaded"),
            serde_json::json!("loaded")
        );
        assert_eq!(
            serde_json::to_value(ModelStatus::Unloaded).expect("serialise unloaded"),
            serde_json::json!("unloaded")
        );
        assert_eq!(
            serde_json::to_value(ModelStatus::Loading).expect("serialise loading"),
            serde_json::json!("loading")
        );

        let unknown = ModelStatus::Unknown("weird".to_string());
        let value = serde_json::to_value(&unknown).expect("serialise unknown");
        assert_eq!(value, serde_json::json!({ "unknown": "weird" }));

        let back: ModelStatus = serde_json::from_value(value).expect("deserialise unknown");
        assert_eq!(back, unknown);
    }

    #[test]
    fn model_without_status_defaults_to_unloaded_and_empty_defaults() {
        let dto: ModelDto = serde_json::from_str(r#"{"id": "bare"}"#).expect("minimal model json");
        let info = ModelInfo::from(dto);

        assert_eq!(info.id, "bare");
        assert_eq!(info.status, ModelStatus::Unloaded);
        assert!(info.status_args.is_empty());
        assert!(info.aliases.is_empty());
        assert!(info.tags.is_empty());
        assert_eq!(info.source, None);
        assert!(!info.can_remove);
        assert_eq!(info.context_length, None);
        assert_eq!(info.architecture, None);
    }

    #[test]
    fn unknown_status_value_is_unknown_variant() {
        let dto: ModelDto = serde_json::from_str(r#"{"id": "m", "status": {"value": "weird"}}"#)
            .expect("unknown status json");
        assert_eq!(
            ModelInfo::from(dto).status,
            ModelStatus::Unknown("weird".to_string())
        );
    }

    #[test]
    fn null_architecture_is_none() {
        let dto: ModelDto = serde_json::from_str(r#"{"id": "m", "architecture": null}"#)
            .expect("null architecture json");
        assert_eq!(ModelInfo::from(dto).architecture, None);
    }

    #[test]
    fn unknown_model_fields_are_tolerated_without_failing() {
        // The contract fixes the ModelInfo shape; unknown server fields (such as
        // `meta` or `object`) must not break the parse.
        let json = r#"{"data": [{
            "id": "m",
            "object": "model",
            "owned_by": "llamacpp",
            "created": 1792099736,
            "meta": {"n_params": 27320697856},
            "status": {"value": "loaded", "args": []}
        }]}"#;
        let dto: ModelsResponseDto = serde_json::from_str(json).expect("lenient model parse");
        let models: Vec<ModelInfo> = dto.data.into_iter().map(ModelInfo::from).collect();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "m");
        assert_eq!(models[0].status, ModelStatus::Loaded);
    }

    #[test]
    fn props_extra_preserves_nested_values_verbatim() {
        let json = r#"{
            "role": "router",
            "custom_list": [1, 2, 3],
            "custom_obj": {"deep": {"x": true}},
            "model_path": "none"
        }"#;
        let dto: PropsDto = serde_json::from_str(json).expect("props json");
        let props: ServerProps = dto.into();

        assert_eq!(
            props.extra.get("custom_list"),
            Some(&serde_json::json!([1, 2, 3]))
        );
        assert_eq!(
            props.extra.get("custom_obj"),
            Some(&serde_json::json!({"deep": {"x": true}}))
        );
        assert_eq!(props.model_path.as_deref(), Some("none"));
    }
}
