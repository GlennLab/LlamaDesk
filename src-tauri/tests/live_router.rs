//! Live integration tests against a running llama.cpp **router**.
//!
//! Every test here is `#[ignore]`d, so a normal `cargo test` (and CI without a
//! server) never runs them. Point them at a live router with:
//!
//! ```text
//! cargo test --test live_router -- --ignored --nocapture
//! ```
//!
//! The base URL defaults to `http://localhost:8080` and can be overridden with
//! the `LLAMADESK_TEST_URL` environment variable.

use std::time::Duration;

use app_lib::domain::error::MonitorError;
use app_lib::domain::model::ModelStatus;
use app_lib::domain::snapshot::DEFAULT_SERVER_URL;
use app_lib::infra::llama::LlamaRouterClient;

/// Resolves the router base URL, defaulting to [`DEFAULT_SERVER_URL`].
fn base_url() -> String {
    std::env::var("LLAMADESK_TEST_URL").unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string())
}

/// Builds a client with an explicit request timeout.
fn client_with_timeout(secs: u64) -> LlamaRouterClient {
    LlamaRouterClient::builder(base_url())
        .expect("base url should be a valid absolute URL")
        .timeout(Duration::from_secs(secs))
        .build()
        .expect("http client should build")
}

/// Builds a client using the default timeout.
fn client() -> LlamaRouterClient {
    client_with_timeout(5)
}

#[tokio::test]
#[ignore = "requires a live llama.cpp router at LLAMADESK_TEST_URL"]
async fn live_server_endpoints_parse() {
    let client = client();

    let health = client.health().await.expect("health should succeed");
    let props = client.props().await.expect("props should succeed");
    let models = client.models().await.expect("models should succeed");

    println!("base_url           = {}", client.base_url());
    println!("health.status      = {}", health.status);
    println!("props.role         = {}", props.role);
    println!("props.build_info   = {:?}", props.build_info);
    println!("models.count       = {}", models.len());

    assert!(!health.status.is_empty(), "health.status should be present");
    assert!(
        !props.role.is_empty(),
        "props.role should be present (router expected)"
    );
}

#[tokio::test]
#[ignore = "requires a live llama.cpp router at LLAMADESK_TEST_URL"]
async fn live_loaded_model_slots_and_metrics_parse() {
    let client = client();
    let models = client.models().await.expect("models should succeed");
    let loaded = models
        .iter()
        .find(|model| model.status == ModelStatus::Loaded)
        .expect("the router is expected to have exactly one loaded model");
    println!("loaded model = {}", loaded.id);

    let slots = client
        .slots(&loaded.id)
        .await
        .expect("slots for the loaded model should succeed");
    println!("slots.count = {}", slots.len());
    for slot in &slots {
        println!(
            "  slot id={} is_processing={} n_ctx={:?} next_token={}",
            slot.id,
            slot.is_processing,
            slot.n_ctx,
            slot.next_token.len()
        );
    }
    assert!(
        !slots.is_empty(),
        "a loaded model should expose at least one decode slot"
    );

    let metrics = client
        .metrics(&loaded.id)
        .await
        .expect("metrics for the loaded model should succeed");
    println!(
        "prompt_tokens_per_sec     = {:?}",
        metrics.prompt_tokens_per_sec
    );
    println!(
        "generation_tokens_per_sec = {:?}",
        metrics.generation_tokens_per_sec
    );
    println!("active_requests           = {:?}", metrics.active_requests);

    assert!(
        metrics.prompt_tokens_per_sec.is_some(),
        "prompt throughput should be present for the loaded model"
    );
    assert!(
        metrics.generation_tokens_per_sec.is_some(),
        "generation throughput should be present for the loaded model"
    );
}

#[tokio::test]
#[ignore = "requires a live llama.cpp router at LLAMADESK_TEST_URL"]
async fn live_qwen_mtp_coder_slots_parse() {
    // Reproduces the exact call that previously failed with
    // `decode error: invalid value: integer '-1', expected u64` on the idle
    // slots' `n_remain`. The model id can be overridden with LLAMADESK_TEST_MODEL.
    let model = std::env::var("LLAMADESK_TEST_MODEL")
        .unwrap_or_else(|_| "qwen-38-27b-mtp-coder".to_string());
    let client = client();

    let slots = client
        .slots(&model)
        .await
        .expect("the qwen mtp-coder slots call that used to fail must now succeed");

    println!("model       = {model}");
    println!("slots.count = {}", slots.len());
    for slot in &slots {
        let n_remain: Vec<Option<i64>> = slot.next_token.iter().map(|t| t.n_remain).collect();
        println!(
            "  slot id={} is_processing={} n_ctx={:?} n_remain={:?}",
            slot.id, slot.is_processing, slot.n_ctx, n_remain
        );
    }

    assert!(!slots.is_empty(), "expected at least one decode slot");
}

#[tokio::test]
#[ignore = "requires a live llama.cpp router at LLAMADESK_TEST_URL"]
async fn live_unloaded_model_slots_never_returns_data() {
    // The frozen contract says an *unloaded* model yields HTTP 200 with an EMPTY
    // body, which the client maps to `Ok(empty)`. That empty-body path is pinned
    // deterministically (no server needed) by
    // `client::tests::empty_body_slots_response_is_ok_empty`.
    //
    // Measured against a live router (llama.cpp b2794, `models_autoload: true`),
    // `/slots?model=<unloaded>` instead BLOCKS: it returned nothing within 35s
    // (curl HTTP 000) without changing any model's status. So the empty-body
    // premise does not hold on this router and a transport error is expected.
    //
    // This test therefore pins the invariant that *is* reachable on any router:
    // an unloaded model must never yield slot data. Empty `Ok` satisfies the
    // contract; a transport error satisfies this router. Anything else fails.
    let client = client_with_timeout(3);
    let models = client.models().await.expect("models should succeed");
    let unloaded = models
        .iter()
        .find(|model| model.status == ModelStatus::Unloaded)
        .expect("the router is expected to list at least one unloaded model");
    println!("unloaded model = {}", unloaded.id);

    match client.slots(&unloaded.id).await {
        Ok(slots) => {
            println!(
                "unloaded /slots -> Ok(empty-body contract met; {} slot(s))",
                slots.len()
            );
            assert!(
                slots.is_empty(),
                "an unloaded model must never report slots, got {slots:?}"
            );
        }
        Err(error @ MonitorError::Timeout) => {
            println!(
                "unloaded /slots -> {error:?}: router blocked (autoload in effect); \
                 empty-body contract not exercised"
            );
        }
        Err(error) if error.is_connection_refused() => {
            println!(
                "unloaded /slots -> {error:?}: transport error; empty-body contract not exercised"
            );
        }
        Err(other) => panic!("unexpected error for an unloaded model: {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires a live llama.cpp router at LLAMADESK_TEST_URL"]
async fn live_unknown_model_slots_reports_http_400() {
    let client = client_with_timeout(3);

    let error = client
        .slots("definitely-not-a-model")
        .await
        .expect_err("an unknown model must not yield slots");

    match error {
        MonitorError::HttpStatus { status, .. } => {
            assert_eq!(status, 400, "router should answer 400 for an unknown model");
            println!("unknown-model /slots -> HTTP {status}");
        }
        MonitorError::NotFound(body) => {
            assert!(
                body.contains("not found"),
                "unexpected NotFound body: {body}"
            );
            println!("unknown-model /slots -> NotFound: {body}");
        }
        other => panic!("expected HTTP 400 or NotFound, got {other:?}"),
    }
}
