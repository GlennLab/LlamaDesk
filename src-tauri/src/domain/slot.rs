//! Types describing the server's decode slots.

use serde::{Deserialize, Serialize};

/// A single entry from `GET /slots?model=<id>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    /// Slot index within the server.
    pub id: i32,
    /// Context size configured for this slot.
    pub n_ctx: Option<u64>,
    /// Whether speculative decoding is enabled for this slot.
    pub speculative: Option<bool>,
    /// Whether the slot is currently processing a request.
    pub is_processing: bool,
    /// Identifier of the task currently assigned to the slot.
    pub id_task: Option<u64>,
    /// Total prompt tokens for the current task.
    pub n_prompt_tokens: Option<u64>,
    /// Prompt tokens already processed.
    pub n_prompt_tokens_processed: Option<u64>,
    /// Prompt tokens served from cache.
    pub n_prompt_tokens_cache: Option<u64>,
    /// Next-token statistics reported for the slot.
    pub next_token: Vec<NextToken>,
    /// Opaque generation parameters, preserved verbatim.
    pub params: Option<serde_json::Value>,
}

/// Next-token statistics reported for a [`Slot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextToken {
    /// Whether another token can be produced.
    pub has_next_token: Option<bool>,
    /// Whether the next token begins a new line.
    pub has_new_line: Option<bool>,
    /// Number of tokens remaining to decode. A negative value (`-1`) means
    /// "no limit / unlimited", reported for idle slots.
    pub n_remain: Option<i64>,
    /// Number of tokens decoded so far.
    pub n_decoded: Option<u64>,
}
