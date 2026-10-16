use super::{health, model, props};
use crate::app::monitor::{probe_once, retention_cutoff_ms};
use crate::domain::error::MonitorError;
use crate::domain::metrics::ServerMetrics;
use crate::domain::model::{ModelInfo, ModelStatus};
use crate::domain::server::{ServerHealth, ServerProps};
use crate::domain::slot::Slot;
use crate::domain::snapshot::ConnectionState;
use crate::ports::provider::InferenceProvider;

/// A provider whose behaviour is fixed at construction, so no real server
/// is required.
struct FakeProvider {
    healthy: bool,
    /// When `true`, `/models` fails even though health/props succeed.
    models_fail: bool,
    /// When set, `health` fails with an [`std::io::Error`] of this kind
    /// (used to model a refused connection); when `None`, it times out.
    failure: Option<std::io::ErrorKind>,
    models: Vec<ModelInfo>,
}

#[async_trait::async_trait]
impl InferenceProvider for FakeProvider {
    async fn health(&self) -> Result<ServerHealth, MonitorError> {
        if self.healthy {
            return Ok(health());
        }
        Err(match self.failure {
            Some(kind) => MonitorError::http(std::io::Error::new(kind, "transport failure")),
            None => MonitorError::Timeout,
        })
    }

    async fn props(&self) -> Result<ServerProps, MonitorError> {
        if self.healthy {
            Ok(props())
        } else {
            Err(MonitorError::Timeout)
        }
    }

    async fn models(&self) -> Result<Vec<ModelInfo>, MonitorError> {
        if self.healthy && !self.models_fail {
            Ok(self.models.clone())
        } else {
            Err(MonitorError::Timeout)
        }
    }

    async fn slots(&self, _model: &str) -> Result<Vec<Slot>, MonitorError> {
        Ok(Vec::new())
    }

    async fn metrics(&self, _model: &str) -> Result<ServerMetrics, MonitorError> {
        Ok(ServerMetrics::default())
    }

    fn display_name(&self) -> &str {
        "fake"
    }
}

#[tokio::test]
async fn probe_once_reports_connected_on_success() {
    let provider = FakeProvider {
        healthy: true,
        models_fail: false,
        failure: None,
        models: vec![model("a", ModelStatus::Loaded)],
    };

    let (state, props, count) = probe_once(&provider).await;

    assert_eq!(state, ConnectionState::Connected);
    assert_eq!(props.map(|p| p.role), Some("router".to_string()));
    assert_eq!(count, Some(1));
}

#[tokio::test]
async fn probe_once_reports_disconnected_on_refused_connection() {
    let provider = FakeProvider {
        healthy: false,
        models_fail: false,
        failure: Some(std::io::ErrorKind::ConnectionRefused),
        models: Vec::new(),
    };

    let (state, props, count) = probe_once(&provider).await;

    assert_eq!(state, ConnectionState::Disconnected);
    assert!(props.is_none());
    assert_eq!(count, None);
}

#[tokio::test]
async fn probe_once_reports_error_on_timeout() {
    let provider = FakeProvider {
        healthy: false,
        models_fail: false,
        failure: None,
        models: Vec::new(),
    };

    let (state, props, count) = probe_once(&provider).await;

    assert_eq!(state, ConnectionState::Error);
    assert!(props.is_none());
    assert_eq!(count, None);
}

#[tokio::test]
async fn probe_once_reports_unknown_model_count_when_models_fail() {
    let provider = FakeProvider {
        healthy: true,
        models_fail: true,
        failure: None,
        models: vec![model("a", ModelStatus::Loaded)],
    };

    let (state, props, count) = probe_once(&provider).await;

    assert_eq!(state, ConnectionState::Connected);
    assert!(props.is_some());
    assert_eq!(count, None, "an unreadable model list is unknown, not zero");
}

#[test]
fn retention_cutoff_subtracts_the_window() {
    assert_eq!(retention_cutoff_ms(1, 7_200_000), 3_600_000);
    assert_eq!(retention_cutoff_ms(24, 86_400_000), 0);
}
