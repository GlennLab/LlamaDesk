//! Typed server metrics and the Prometheus exposition data they are derived from.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A single Prometheus sample's label set.
pub type LabelSet = BTreeMap<String, String>;

/// Parsed contents of a Prometheus text exposition.
///
/// Values are stored verbatim under their declared metric name (the llama.cpp
/// `llamacpp:` prefix is preserved). Unknown or untyped series are recorded as
/// gauges so that no data is silently dropped.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusMetrics {
    /// Last value seen for each gauge, keyed by metric name.
    pub gauges: BTreeMap<String, f64>,
    /// Last value seen for each counter, keyed by metric name.
    pub counters: BTreeMap<String, f64>,
    /// Every sample seen, grouped by metric name (labels and value).
    pub by_name: BTreeMap<String, Vec<(LabelSet, f64)>>,
}

/// Base metric names (without the `llamacpp:` prefix) that map to a typed field.
const TYPED_METRIC_NAMES: [&str; 9] = [
    "prompt_tokens_seconds",
    "predicted_tokens_seconds",
    "requests_processing",
    "requests_deferred",
    "prompt_tokens_total",
    "prompt_tokens_cached_total",
    "tokens_predicted_total",
    "n_decode_total",
    "n_tokens_max",
];

/// Normalized, typed view of the server metrics shown in the UI.
///
/// Every field is optional so that a missing metric stays distinguishable from
/// a real zero. `raw` retains any gauges that do not have a typed field.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerMetrics {
    /// Prompt processing throughput (tokens/sec).
    pub prompt_tokens_per_sec: Option<f64>,
    /// Generation throughput (tokens/sec).
    pub generation_tokens_per_sec: Option<f64>,
    /// Number of requests currently being processed.
    pub active_requests: Option<f64>,
    /// Number of deferred requests.
    pub deferred_requests: Option<f64>,
    /// Cumulative prompt tokens processed.
    pub prompt_tokens_total: Option<f64>,
    /// Cumulative prompt tokens served from cache.
    pub prompt_tokens_cached_total: Option<f64>,
    /// Cumulative predicted (generated) tokens.
    pub tokens_predicted_total: Option<f64>,
    /// Cumulative number of decode iterations.
    pub n_decode_total: Option<f64>,
    /// Maximum tokens observed in a single sequence.
    pub n_tokens_max: Option<f64>,
    /// Derived KV-cache usage ratio (`cached / total`), when both are present.
    pub kv_cache_usage_ratio: Option<f64>,
    /// Unmapped gauges, keyed by their raw metric name.
    pub raw: BTreeMap<String, f64>,
}

impl ServerMetrics {
    /// Builds a typed view from parsed Prometheus metrics.
    ///
    /// Metric names are looked up both with and without the `llamacpp:` prefix,
    /// and in both the gauge and counter maps, so the mapping is resilient to
    /// how the server declares each series.
    pub fn from_prometheus(metrics: &PrometheusMetrics) -> Self {
        let lookup = |name: &str| -> Option<f64> {
            let prefixed = format!("llamacpp:{name}");
            metrics
                .gauges
                .get(name)
                .copied()
                .or_else(|| metrics.counters.get(name).copied())
                .or_else(|| metrics.gauges.get(&prefixed).copied())
                .or_else(|| metrics.counters.get(&prefixed).copied())
        };

        let prompt_tokens_total = lookup("prompt_tokens_total");
        let prompt_tokens_cached_total = lookup("prompt_tokens_cached_total");
        let kv_cache_usage_ratio = match (prompt_tokens_total, prompt_tokens_cached_total) {
            (Some(total), Some(cached)) if total > 0.0 => Some(cached / total),
            _ => None,
        };

        let mut raw = BTreeMap::new();
        for (name, value) in &metrics.gauges {
            let base = name.strip_prefix("llamacpp:").unwrap_or(name.as_str());
            if !TYPED_METRIC_NAMES.contains(&base) {
                raw.insert(name.clone(), *value);
            }
        }

        Self {
            prompt_tokens_per_sec: lookup("prompt_tokens_seconds"),
            generation_tokens_per_sec: lookup("predicted_tokens_seconds"),
            active_requests: lookup("requests_processing"),
            deferred_requests: lookup("requests_deferred"),
            prompt_tokens_total,
            prompt_tokens_cached_total,
            tokens_predicted_total: lookup("tokens_predicted_total"),
            n_decode_total: lookup("n_decode_total"),
            n_tokens_max: lookup("n_tokens_max"),
            kv_cache_usage_ratio,
            raw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pm(gauges: &[(&str, f64)], counters: &[(&str, f64)]) -> PrometheusMetrics {
        let mut metrics = PrometheusMetrics::default();
        for (k, v) in gauges {
            metrics.gauges.insert((*k).to_string(), *v);
        }
        for (k, v) in counters {
            metrics.counters.insert((*k).to_string(), *v);
        }
        metrics
    }

    #[test]
    fn absent_metrics_stay_none_instead_of_zero() {
        let metrics = pm(&[], &[]);
        let out = ServerMetrics::from_prometheus(&metrics);
        assert_eq!(out.prompt_tokens_per_sec, None);
        assert_eq!(out.generation_tokens_per_sec, None);
        assert_eq!(out.active_requests, None);
        assert_eq!(out.deferred_requests, None);
        assert_eq!(out.kv_cache_usage_ratio, None);
        assert!(out.raw.is_empty());
    }

    #[test]
    fn kv_cache_ratio_is_derived_from_cached_and_total() {
        let metrics = pm(
            &[],
            &[
                ("llamacpp:prompt_tokens_total", 100.0),
                ("llamacpp:prompt_tokens_cached_total", 25.0),
            ],
        );
        let out = ServerMetrics::from_prometheus(&metrics);
        assert_eq!(out.kv_cache_usage_ratio, Some(0.25));
    }

    #[test]
    fn kv_cache_ratio_is_none_when_total_is_zero() {
        let metrics = pm(
            &[],
            &[
                ("llamacpp:prompt_tokens_total", 0.0),
                ("llamacpp:prompt_tokens_cached_total", 5.0),
            ],
        );
        let out = ServerMetrics::from_prometheus(&metrics);
        assert_eq!(out.kv_cache_usage_ratio, None);
    }

    #[test]
    fn untyped_gauges_land_in_raw() {
        let metrics = pm(
            &[
                ("llamacpp:requests_processing", 2.0),
                ("llamacpp:custom_gauge", 7.0),
            ],
            &[],
        );
        let out = ServerMetrics::from_prometheus(&metrics);
        assert_eq!(out.active_requests, Some(2.0));
        assert_eq!(out.raw.get("llamacpp:custom_gauge"), Some(&7.0));
        assert!(!out.raw.contains_key("llamacpp:requests_processing"));
    }
}
