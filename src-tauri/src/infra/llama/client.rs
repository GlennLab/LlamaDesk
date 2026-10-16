//! HTTP client for a live llama.cpp **router** server.
//!
//! All calls use an explicit timeout and never panic. `/slots` and `/metrics`
//! require a `?model=<id>` query parameter; an empty 200 body is treated as "no
//! data" rather than an error.

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Url;

use crate::domain::error::MonitorError;
use crate::domain::metrics::ServerMetrics;
use crate::domain::model::ModelInfo;
use crate::domain::server::{ServerHealth, ServerProps};
use crate::domain::slot::Slot;
use crate::infra::llama::dto::{HealthDto, ModelsResponseDto, PropsDto, SlotDto};
use crate::infra::llama::prometheus;
use crate::ports::provider::InferenceProvider;

/// Default request timeout for router calls.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
/// Default TCP connect timeout for router calls.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Read-only client for the llama.cpp router HTTP API.
#[derive(Debug, Clone)]
pub struct LlamaRouterClient {
    base_url: Url,
    http: reqwest::Client,
}

/// Configures and builds a [`LlamaRouterClient`].
#[derive(Debug)]
pub struct LlamaRouterClientBuilder {
    base_url: Url,
    timeout: Duration,
    connect_timeout: Duration,
}

impl LlamaRouterClient {
    /// Starts building a client for `base_url` (e.g. `http://localhost:8080`).
    pub fn builder(base_url: impl AsRef<str>) -> Result<LlamaRouterClientBuilder, MonitorError> {
        let base_url = Url::parse(base_url.as_ref())
            .map_err(|error| MonitorError::Config(format!("invalid base url: {error}")))?;
        Ok(LlamaRouterClientBuilder {
            base_url,
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
        })
    }

    /// The configured base URL.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Returns the server health.
    pub async fn health(&self) -> Result<ServerHealth, MonitorError> {
        let body = self
            .fetch(build_url(&self.base_url, "health", None)?)
            .await?;
        let dto: HealthDto = serde_json::from_str(&body).map_err(MonitorError::decode)?;
        Ok(dto.into())
    }

    /// Returns the server properties.
    pub async fn props(&self) -> Result<ServerProps, MonitorError> {
        let body = self
            .fetch(build_url(&self.base_url, "props", None)?)
            .await?;
        let dto: PropsDto = serde_json::from_str(&body).map_err(MonitorError::decode)?;
        Ok(dto.into())
    }

    /// Returns the models known to the server.
    pub async fn models(&self) -> Result<Vec<ModelInfo>, MonitorError> {
        let body = self
            .fetch(build_url(&self.base_url, "models", None)?)
            .await?;
        let dto: ModelsResponseDto = serde_json::from_str(&body).map_err(MonitorError::decode)?;
        Ok(dto.data.into_iter().map(ModelInfo::from).collect())
    }

    /// Returns the decode slots for `model` (empty when the model is unloaded).
    pub async fn slots(&self, model: &str) -> Result<Vec<Slot>, MonitorError> {
        let body = self
            .fetch(build_url(&self.base_url, "slots", Some(model))?)
            .await?;
        parse_slots_body(&body)
    }

    /// Returns the metrics for `model`.
    pub async fn metrics(&self, model: &str) -> Result<ServerMetrics, MonitorError> {
        let body = self
            .fetch(build_url(&self.base_url, "metrics", Some(model))?)
            .await?;
        if body.trim().is_empty() {
            return Ok(ServerMetrics::default());
        }
        Ok(ServerMetrics::from_prometheus(&prometheus::parse(&body)))
    }

    /// Performs a GET, mapping non-success responses to [`MonitorError`].
    async fn fetch(&self, url: Url) -> Result<String, MonitorError> {
        let response = self.http.get(url).send().await.map_err(map_reqwest)?;
        let status = response.status();
        let body = response.text().await.map_err(map_reqwest)?;
        if !status.is_success() {
            return Err(map_status(status.as_u16(), body));
        }
        Ok(body)
    }
}

impl LlamaRouterClientBuilder {
    /// Overrides the total request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Overrides the TCP connect timeout.
    pub fn connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = connect_timeout;
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<LlamaRouterClient, MonitorError> {
        let http = reqwest::Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .build()
            .map_err(|error| {
                MonitorError::Config(format!("failed to build http client: {error}"))
            })?;
        Ok(LlamaRouterClient {
            base_url: self.base_url,
            http,
        })
    }
}

#[async_trait]
impl InferenceProvider for LlamaRouterClient {
    async fn health(&self) -> Result<ServerHealth, MonitorError> {
        LlamaRouterClient::health(self).await
    }

    async fn props(&self) -> Result<ServerProps, MonitorError> {
        LlamaRouterClient::props(self).await
    }

    async fn models(&self) -> Result<Vec<ModelInfo>, MonitorError> {
        LlamaRouterClient::models(self).await
    }

    async fn slots(&self, model: &str) -> Result<Vec<Slot>, MonitorError> {
        LlamaRouterClient::slots(self, model).await
    }

    async fn metrics(&self, model: &str) -> Result<ServerMetrics, MonitorError> {
        LlamaRouterClient::metrics(self, model).await
    }

    fn display_name(&self) -> &str {
        "llama.cpp router"
    }
}

/// Builds an absolute URL for `path`, appending `?model=<id>` when requested.
///
/// Tolerates a base URL with or without a trailing slash and a `path` with or
/// without a leading slash, so no double slashes are ever produced.
fn build_url(base_url: &Url, path: &str, model: Option<&str>) -> Result<Url, MonitorError> {
    let mut base = base_url.clone();
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    let mut url = base
        .join(path.trim_start_matches('/'))
        .map_err(|error| MonitorError::Config(format!("invalid request path: {error}")))?;
    if let Some(model) = model {
        url.query_pairs_mut().append_pair("model", model);
    }
    Ok(url)
}

/// Parses a `/slots` body, treating an empty body as an empty slot list.
///
/// Each array entry is deserialized independently, so a single malformed entry
/// is skipped rather than failing the whole response; the remaining slots are
/// still returned.
fn parse_slots_body(body: &str) -> Result<Vec<Slot>, MonitorError> {
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(body).map_err(MonitorError::decode)?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| serde_json::from_value::<SlotDto>(entry).ok())
        .map(Slot::from)
        .collect())
}

/// Maps a `reqwest` failure to the closest [`MonitorError`] variant.
fn map_reqwest(error: reqwest::Error) -> MonitorError {
    if error.is_timeout() {
        MonitorError::Timeout
    } else if error.is_decode() {
        MonitorError::decode(error)
    } else {
        MonitorError::http(error)
    }
}

/// Maps an HTTP status code and body to the closest [`MonitorError`] variant.
fn map_status(status: u16, body: String) -> MonitorError {
    if status == 404 {
        MonitorError::NotFound(body)
    } else {
        MonitorError::HttpStatus { status, body }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_appends_model_query() {
        let base = Url::parse("http://localhost:8080").expect("base url");
        let url = build_url(&base, "slots", Some("my model/x")).expect("url");
        assert_eq!(url.path(), "/slots");
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        assert_eq!(pairs, vec![("model".to_string(), "my model/x".to_string())]);
    }

    #[test]
    fn build_url_omits_query_when_no_model() {
        let base = Url::parse("http://localhost:8080/").expect("base url");
        let url = build_url(&base, "models", None).expect("url");
        assert_eq!(url.query(), None);
        assert_eq!(url.path(), "/models");
    }

    #[test]
    fn build_url_normalizes_slashes() {
        let no_slash = build_url(
            &Url::parse("http://localhost:8080").expect("base url"),
            "health",
            None,
        )
        .expect("url");
        assert_eq!(no_slash.as_str(), "http://localhost:8080/health");

        let with_slash = build_url(
            &Url::parse("http://localhost:8080/").expect("base url"),
            "/health",
            None,
        )
        .expect("url");
        assert_eq!(with_slash.as_str(), "http://localhost:8080/health");
    }

    #[test]
    fn build_url_preserves_base_subpath() {
        let url = build_url(
            &Url::parse("http://localhost:8080/api").expect("base url"),
            "props",
            None,
        )
        .expect("url");
        assert_eq!(url.as_str(), "http://localhost:8080/api/props");
    }

    #[test]
    fn empty_slots_body_is_empty_vec() {
        assert!(parse_slots_body("").expect("empty body").is_empty());
        assert!(parse_slots_body("  \n\t ").expect("blank body").is_empty());
    }

    #[test]
    fn slots_body_parses_array() {
        let slots = parse_slots_body(r#"[{"id":0,"is_processing":true}]"#).expect("slots");
        assert_eq!(slots.len(), 1);
        assert!(slots[0].is_processing);
    }

    #[test]
    fn slots_body_skips_malformed_entry() {
        // The first entry has an unparseable `id`; it must be dropped without
        // failing the whole response, so the second, valid entry is still returned.
        let slots = parse_slots_body(r#"[{"id":"not-a-number"},{"id":1,"is_processing":true}]"#)
            .expect("slots");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].id, 1);
        assert!(slots[0].is_processing);
    }

    #[test]
    fn map_status_classifies_not_found() {
        assert!(matches!(
            map_status(404, "missing".to_string()),
            MonitorError::NotFound(_)
        ));
        match map_status(400, "bad".to_string()) {
            MonitorError::HttpStatus { status, body } => {
                assert_eq!(status, 400);
                assert_eq!(body, "bad");
            }
            other => panic!("expected HttpStatus, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn maps_400_with_body_to_http_status() {
        use tokio::io::AsyncWriteExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let body = r#"{"error":{"code":400,"message":"model name is missing from the request","type":"invalid_request_error"}}"#;
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let server = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });

        let client = LlamaRouterClient::builder(format!("http://{addr}"))
            .expect("builder")
            .build()
            .expect("client");
        let error = client.slots("some-model").await.expect_err("should be 400");
        server.await.expect("server task");

        match error {
            MonitorError::HttpStatus { status, body } => {
                assert_eq!(status, 400);
                assert!(body.contains("model name is missing"));
            }
            other => panic!("expected HttpStatus, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn connect_failure_is_reported_as_connection_error() {
        let client = LlamaRouterClient::builder("http://127.0.0.1:1")
            .expect("builder")
            .timeout(Duration::from_millis(300))
            .connect_timeout(Duration::from_millis(300))
            .build()
            .expect("client");
        let error = client.health().await.expect_err("should fail");
        assert!(error.is_connection_refused(), "unexpected error: {error:?}");
    }

    #[tokio::test]
    async fn empty_body_slots_response_is_ok_empty() {
        use tokio::io::AsyncWriteExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let server = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });

        let client = LlamaRouterClient::builder(format!("http://{addr}"))
            .expect("builder")
            .build()
            .expect("client");
        let slots = client
            .slots("unloaded-model")
            .await
            .expect("an empty 200 body must be Ok(empty), not an error");
        server.await.expect("server task");

        assert!(slots.is_empty());
    }

    #[tokio::test]
    async fn empty_body_metrics_response_is_default() {
        use tokio::io::AsyncWriteExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let server = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });

        let client = LlamaRouterClient::builder(format!("http://{addr}"))
            .expect("builder")
            .build()
            .expect("client");
        let metrics = client
            .metrics("unloaded-model")
            .await
            .expect("an empty 200 body must be Ok(default), not an error");
        server.await.expect("server task");

        assert_eq!(metrics, ServerMetrics::default());
    }
}
