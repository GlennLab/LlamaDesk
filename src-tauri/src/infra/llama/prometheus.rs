//! Minimal, dependency-free Prometheus text-exposition parser.
//!
//! Handles `# HELP`/`# TYPE` comments, `name value` and `name{label="v"} value`
//! samples, scientific notation, and trailing whitespace. Malformed lines are
//! skipped rather than failing the whole parse.

use std::collections::BTreeMap;

pub use crate::domain::metrics::PrometheusMetrics;

/// Parses a Prometheus text exposition into [`PrometheusMetrics`].
///
/// Only genuinely empty input yields an empty result; individual malformed
/// lines are ignored so a single bad sample never discards the rest.
pub fn parse(input: &str) -> PrometheusMetrics {
    let mut metrics = PrometheusMetrics::default();
    let mut types: BTreeMap<String, String> = BTreeMap::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            parse_type(rest.trim(), &mut types);
            continue;
        }
        parse_sample(line, &types, &mut metrics);
    }

    metrics
}

/// Records a `TYPE <name> <kind>` declaration. All other comments are ignored.
fn parse_type(rest: &str, types: &mut BTreeMap<String, String>) {
    let mut parts = rest.split_whitespace();
    if parts.next() != Some("TYPE") {
        return;
    }
    if let (Some(name), Some(kind)) = (parts.next(), parts.next()) {
        types.insert(name.to_string(), kind.to_string());
    }
}

/// Parses one `name{labels} value` sample line, skipping it if malformed.
fn parse_sample(line: &str, types: &BTreeMap<String, String>, metrics: &mut PrometheusMetrics) {
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() && bytes[index] != b'{' && !bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let name = &line[..index];
    if name.is_empty() {
        return;
    }

    let mut labels = BTreeMap::new();
    if index < bytes.len() && bytes[index] == b'{' {
        let Some(end) = matching_brace(bytes, index) else {
            return;
        };
        parse_labels(&line[index + 1..end], &mut labels);
        index = end + 1;
    }

    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let Some(value) = line[index..].split_whitespace().next() else {
        return;
    };
    let Ok(value) = value.parse::<f64>() else {
        return;
    };

    metrics
        .by_name
        .entry(name.to_string())
        .or_default()
        .push((labels, value));

    let kind = types.get(name).map(String::as_str).unwrap_or("gauge");
    if kind == "counter" {
        metrics.counters.insert(name.to_string(), value);
    } else {
        metrics.gauges.insert(name.to_string(), value);
    }
}

/// Returns the byte index of the `}` matching the `{` at `open`, honouring
/// quoted label values.
fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut index = open + 1;
    let mut in_quotes = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_quotes {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_quotes = false;
            }
        } else if byte == b'"' {
            in_quotes = true;
        } else if byte == b'}' {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// Parses a `k="v",k2="v2"` label block into `out`.
fn parse_labels(input: &str, out: &mut BTreeMap<String, String>) {
    for pair in split_top_level_commas(input) {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some(eq) = pair.find('=') {
            let key = pair[..eq].trim().to_string();
            let value = pair[eq + 1..].trim().trim_matches('"').to_string();
            out.insert(key, value);
        }
    }
}

/// Splits on commas that are not inside a quoted label value.
fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut escaped = false;
    for (index, byte) in input.bytes().enumerate() {
        if in_quotes {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_quotes = false;
            }
        } else if byte == b'"' {
            in_quotes = true;
        } else if byte == b',' {
            parts.push(&input[start..index]);
            start = index + 1;
        }
    }
    parts.push(&input[start..]);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::metrics::ServerMetrics;

    const SAMPLE: &str = r#"
# HELP llamacpp:prompt_tokens_seconds Prompt processing speed.
# TYPE llamacpp:prompt_tokens_seconds gauge
llamacpp:prompt_tokens_seconds 4.36529e+02
# TYPE llamacpp:requests_processing gauge
llamacpp:requests_processing 2
# TYPE llamacpp:requests_deferred gauge
llamacpp:requests_deferred 0
# TYPE llamacpp:predicted_tokens_seconds gauge
llamacpp:predicted_tokens_seconds 31.5
# TYPE llamacpp:prompt_tokens_total counter
llamacpp:prompt_tokens_total 4.36529e+06
# TYPE llamacpp:prompt_tokens_cached_total counter
llamacpp:prompt_tokens_cached_total 1.0e+06
# TYPE llamacpp:spec_decode_num_accepted_tokens_per_pos_total counter
llamacpp:spec_decode_num_accepted_tokens_per_pos_total{position="0"} 12
llamacpp:spec_decode_num_accepted_tokens_per_pos_total{position="1"} 7
this line is malformed
llamacpp:garbage not-a-number
"#;

    #[test]
    fn parses_gauges_counters_and_scientific_notation() {
        let metrics = parse(SAMPLE);
        assert_eq!(
            metrics.gauges.get("llamacpp:prompt_tokens_seconds"),
            Some(&436.529)
        );
        assert_eq!(
            metrics.gauges.get("llamacpp:requests_processing"),
            Some(&2.0)
        );
        assert_eq!(
            metrics.gauges.get("llamacpp:predicted_tokens_seconds"),
            Some(&31.5)
        );
        assert_eq!(
            metrics.counters.get("llamacpp:prompt_tokens_total"),
            Some(&4_365_290.0)
        );
        assert_eq!(
            metrics.counters.get("llamacpp:prompt_tokens_cached_total"),
            Some(&1_000_000.0)
        );
    }

    #[test]
    fn records_labelled_samples_in_by_name() {
        let metrics = parse(SAMPLE);
        let series = metrics
            .by_name
            .get("llamacpp:spec_decode_num_accepted_tokens_per_pos_total")
            .expect("position series");
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].0.get("position").map(String::as_str), Some("0"));
        assert_eq!(series[0].1, 12.0);
        assert_eq!(series[1].0.get("position").map(String::as_str), Some("1"));
        assert_eq!(series[1].1, 7.0);
    }

    #[test]
    fn skips_malformed_lines_without_failing() {
        let metrics = parse(SAMPLE);
        assert!(!metrics.by_name.contains_key("this line is malformed"));
        assert!(!metrics.by_name.contains_key("llamacpp:garbage"));
        // The well-formed samples before and after the bad lines survive.
        assert!(
            metrics
                .gauges
                .contains_key("llamacpp:predicted_tokens_seconds")
        );
    }

    #[test]
    fn handles_comments_blank_lines_and_crlf() {
        let metrics = parse("#just a comment\r\n\r\nllamacpp:x 1.5   \r\n");
        assert_eq!(metrics.gauges.get("llamacpp:x"), Some(&1.5));
    }

    #[test]
    fn untyped_metric_defaults_to_gauge() {
        let metrics = parse("llamacpp:no_type_here 5\n");
        assert_eq!(metrics.gauges.get("llamacpp:no_type_here"), Some(&5.0));
        assert!(metrics.counters.is_empty());
    }

    #[test]
    fn from_prometheus_maps_known_metrics_and_leaves_absent_as_none() {
        let metrics = parse(SAMPLE);
        let typed = ServerMetrics::from_prometheus(&metrics);

        assert_eq!(typed.prompt_tokens_per_sec, Some(436.529));
        assert_eq!(typed.generation_tokens_per_sec, Some(31.5));
        assert_eq!(typed.active_requests, Some(2.0));
        assert_eq!(typed.deferred_requests, Some(0.0));
        assert_eq!(typed.prompt_tokens_total, Some(4_365_290.0));
        assert_eq!(typed.prompt_tokens_cached_total, Some(1_000_000.0));
        // Not present in the exposition -> None, never 0.
        assert_eq!(typed.n_decode_total, None);
        assert_eq!(typed.tokens_predicted_total, None);
        // Derived ratio from cached/total.
        assert_eq!(typed.kv_cache_usage_ratio, Some(1_000_000.0 / 4_365_290.0));
    }

    #[test]
    fn empty_input_yields_empty_metrics() {
        let metrics = parse("");
        assert!(metrics.gauges.is_empty());
        assert!(metrics.counters.is_empty());
        assert!(metrics.by_name.is_empty());
    }

    #[test]
    fn duplicate_metric_name_keeps_all_samples_but_last_value_wins() {
        let metrics = parse("llamacpp:dup 1\nllamacpp:dup 2\n");
        assert_eq!(metrics.by_name.get("llamacpp:dup").map(Vec::len), Some(2));
        assert_eq!(metrics.gauges.get("llamacpp:dup"), Some(&2.0));
    }

    #[test]
    fn type_and_help_without_samples_leave_metrics_empty() {
        let metrics = parse(
            "# HELP llamacpp:ghost A metric that never reports a sample\n\
             # TYPE llamacpp:ghost gauge\n",
        );
        assert!(metrics.gauges.is_empty());
        assert!(metrics.counters.is_empty());
        assert!(metrics.by_name.is_empty());
    }

    #[test]
    fn parses_infinity_and_nan_values() {
        let metrics =
            parse("llamacpp:pos_inf +Inf\nllamacpp:neg_inf -Inf\nllamacpp:not_a_number NaN\n");
        assert_eq!(metrics.gauges.get("llamacpp:pos_inf"), Some(&f64::INFINITY));
        assert_eq!(
            metrics.gauges.get("llamacpp:neg_inf"),
            Some(&f64::NEG_INFINITY)
        );
        assert!(
            metrics
                .gauges
                .get("llamacpp:not_a_number")
                .expect("nan")
                .is_nan()
        );
    }

    #[test]
    fn label_value_may_contain_brace_and_escaped_quote() {
        let metrics = parse("llamacpp:labelled{msg=\"has } brace\",q=\"quote\\\"inside\"} 3\n");
        let series = metrics
            .by_name
            .get("llamacpp:labelled")
            .expect("labelled series");
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].1, 3.0);
        // A `}` inside a quoted value must not terminate the label block.
        assert_eq!(
            series[0].0.get("msg").map(String::as_str),
            Some("has } brace")
        );
        // The escaped quote keeps the value in one label; the raw escape is kept.
        assert_eq!(
            series[0].0.get("q").map(String::as_str),
            Some("quote\\\"inside")
        );
    }

    #[test]
    fn crlf_with_labels_parses() {
        let metrics = parse("llamacpp:counter{position=\"0\"} 9\r\n");
        assert_eq!(metrics.gauges.get("llamacpp:counter"), Some(&9.0));
        let series = metrics.by_name.get("llamacpp:counter").expect("series");
        assert_eq!(series[0].0.get("position").map(String::as_str), Some("0"));
    }

    #[test]
    fn trailing_spaces_after_value_are_ignored() {
        let metrics = parse("llamacpp:spaced 42   \n");
        assert_eq!(metrics.gauges.get("llamacpp:spaced"), Some(&42.0));
    }

    #[test]
    fn name_without_value_is_skipped() {
        let metrics = parse("llamacpp:novalue\n");
        assert!(!metrics.by_name.contains_key("llamacpp:novalue"));
        assert!(metrics.gauges.is_empty());
        assert!(metrics.counters.is_empty());
    }
}
