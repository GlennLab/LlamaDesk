//! One-shot connection probe used by the `test_connection` command, plus the
//! retention-window helper shared with the storage loop.

use crate::domain::server::ServerProps;
use crate::domain::snapshot::ConnectionState;
use crate::ports::provider::InferenceProvider;

use super::classify_connection_error;

/// Milliseconds in one hour.
const MS_PER_HOUR: i64 = 3_600_000;

/// Returns the oldest timestamp (epoch ms) retained for a `retention_hours`
/// window ending at `now_ms`.
pub fn retention_cutoff_ms(retention_hours: u64, now_ms: i64) -> i64 {
    now_ms - retention_hours as i64 * MS_PER_HOUR
}

/// Performs a single connection probe against `provider`.
///
/// Returns the resulting connection state, the server properties (when the
/// probe reached `/props`), and the number of models reported. The model count
/// is `None` when `/models` could not be read, so callers can distinguish "no
/// models" from "unknown" rather than being told zero. Never panics.
pub async fn probe_once(
    provider: &dyn InferenceProvider,
) -> (ConnectionState, Option<ServerProps>, Option<usize>) {
    match provider.health().await {
        Ok(_) => {}
        Err(error) => return (classify_connection_error(&error), None, None),
    }

    let props = provider.props().await.ok();
    let model_count = provider.models().await.map(|models| models.len()).ok();
    (ConnectionState::Connected, props, model_count)
}
