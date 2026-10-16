use std::collections::BTreeMap;

use super::{health, model, props, slot};
use crate::app::monitor::{
    ModelPoll, SLICE_HARDWARE, SLICE_METRICS, SLICE_SERVER, SLICE_SLOTS, ServerSlice,
    apply_hardware_result, apply_model_results, apply_server_result, classify_connection_error,
    loaded_model_ids,
};
use crate::domain::error::MonitorError;
use crate::domain::hardware::GpuStats;
use crate::domain::metrics::ServerMetrics;
use crate::domain::model::ModelStatus;
use crate::domain::snapshot::{ConnectionState, MonitorSnapshot};

#[test]
fn classify_maps_connection_errors() {
    assert_eq!(
        classify_connection_error(&MonitorError::Timeout),
        ConnectionState::Error
    );
    assert_eq!(
        classify_connection_error(&MonitorError::http(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused"
        ))),
        ConnectionState::Disconnected
    );
    assert_eq!(
        classify_connection_error(&MonitorError::HttpStatus {
            status: 500,
            body: String::new(),
        }),
        ConnectionState::Error
    );
    assert_eq!(
        classify_connection_error(&MonitorError::Config("bad".to_string())),
        ConnectionState::Error
    );
}

#[test]
fn loaded_model_ids_filters_by_status() {
    let models = vec![
        model("a", ModelStatus::Loaded),
        model("b", ModelStatus::Unloaded),
        model("c", ModelStatus::Loading),
        model("d", ModelStatus::Loaded),
        model("e", ModelStatus::Unknown("weird".to_string())),
    ];

    assert_eq!(
        loaded_model_ids(&models),
        vec!["a".to_string(), "d".to_string()]
    );
}

#[test]
fn apply_server_result_success_sets_connected_and_clears_error() {
    let mut snapshot = MonitorSnapshot::default();
    snapshot.set_slice_error(SLICE_SERVER, "stale".to_string());
    let slice = ServerSlice {
        health: health(),
        props: props(),
        models: vec![
            model("a", ModelStatus::Loaded),
            model("b", ModelStatus::Unloaded),
        ],
    };

    apply_server_result(&mut snapshot, Ok(slice));

    assert_eq!(snapshot.connection, ConnectionState::Connected);
    assert_eq!(snapshot.loaded_models, vec!["a".to_string()]);
    assert!(snapshot.errors.is_empty());
    assert!(snapshot.last_updated.is_some());
    assert_eq!(
        snapshot.props.as_ref().map(|p| p.role.as_str()),
        Some("router")
    );
}

#[test]
fn apply_server_result_failure_preserves_previous_data() {
    let mut snapshot = MonitorSnapshot {
        props: Some(props()),
        models: vec![model("a", ModelStatus::Loaded)],
        loaded_models: vec!["a".to_string()],
        connection: ConnectionState::Connected,
        ..MonitorSnapshot::default()
    };

    apply_server_result(
        &mut snapshot,
        Err(MonitorError::http(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused",
        ))),
    );

    assert_eq!(snapshot.connection, ConnectionState::Disconnected);
    assert!(snapshot.props.is_some(), "props must be preserved");
    assert_eq!(snapshot.models.len(), 1, "models must be preserved");
    assert_eq!(snapshot.loaded_models, vec!["a".to_string()]);
    assert!(snapshot.errors.iter().any(|e| e.slice == SLICE_SERVER));
    assert!(snapshot.last_updated.is_some());
}

#[test]
fn apply_server_result_timeout_is_an_error_not_disconnected() {
    let mut snapshot = MonitorSnapshot {
        connection: ConnectionState::Connected,
        ..MonitorSnapshot::default()
    };

    apply_server_result(&mut snapshot, Err(MonitorError::Timeout));

    assert_eq!(snapshot.connection, ConnectionState::Error);
}

#[test]
fn connection_loss_after_success_keeps_last_known_server_data() {
    let mut snapshot = MonitorSnapshot::default();

    // Establish a good baseline.
    apply_server_result(
        &mut snapshot,
        Ok(ServerSlice {
            health: health(),
            props: props(),
            models: vec![
                model("a", ModelStatus::Loaded),
                model("b", ModelStatus::Unloaded),
            ],
        }),
    );
    assert_eq!(snapshot.connection, ConnectionState::Connected);
    assert_eq!(snapshot.loaded_models, vec!["a".to_string()]);

    // Server answered but misbehaved: connection is Error, data retained.
    apply_server_result(
        &mut snapshot,
        Err(MonitorError::HttpStatus {
            status: 500,
            body: String::new(),
        }),
    );
    assert_eq!(snapshot.connection, ConnectionState::Error);
    assert!(snapshot.health.is_some());
    assert!(snapshot.props.is_some());
    assert_eq!(snapshot.models.len(), 2);
    assert_eq!(snapshot.loaded_models, vec!["a".to_string()]);

    // Server unreachable: Disconnected, still retained.
    apply_server_result(
        &mut snapshot,
        Err(MonitorError::http(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused",
        ))),
    );
    assert_eq!(snapshot.connection, ConnectionState::Disconnected);
    assert!(snapshot.props.is_some());
    assert_eq!(snapshot.models.len(), 2);
    assert_eq!(snapshot.loaded_models, vec!["a".to_string()]);

    // Recovery clears the server error and refreshes the data.
    apply_server_result(
        &mut snapshot,
        Ok(ServerSlice {
            health: health(),
            props: props(),
            models: vec![model("a", ModelStatus::Loaded)],
        }),
    );
    assert_eq!(snapshot.connection, ConnectionState::Connected);
    assert_eq!(snapshot.models.len(), 1);
    assert!(
        snapshot.errors.iter().all(|e| e.slice != SLICE_SERVER),
        "a successful poll must clear the server error"
    );
}

#[test]
fn apply_model_results_drops_unloaded_and_imports_polled_models() {
    let mut snapshot = MonitorSnapshot::default();
    snapshot.slots.insert("stale".to_string(), Vec::new());
    let loaded = vec!["a".to_string(), "b".to_string()];
    let mut results = BTreeMap::new();
    results.insert(
        "a".to_string(),
        ModelPoll {
            slots: Ok(Vec::new()),
            metrics: Ok(ServerMetrics::default()),
        },
    );
    results.insert(
        "b".to_string(),
        ModelPoll {
            slots: Err(MonitorError::Timeout),
            metrics: Ok(ServerMetrics::default()),
        },
    );

    apply_model_results(&mut snapshot, &loaded, &results);

    assert!(snapshot.slots.contains_key("a"));
    assert!(!snapshot.slots.contains_key("b"));
    assert!(!snapshot.slots.contains_key("stale"));
    assert!(snapshot.metrics.contains_key("a"));
    assert!(snapshot.errors.iter().any(|e| e.slice == SLICE_SLOTS));
}

#[test]
fn apply_model_results_keeps_last_known_value_on_failure() {
    let mut snapshot = MonitorSnapshot::default();
    let prior = vec![slot(0)];
    snapshot.slots.insert("a".to_string(), prior.clone());
    snapshot
        .metrics
        .insert("a".to_string(), ServerMetrics::default());
    let loaded = vec!["a".to_string()];
    let mut results = BTreeMap::new();
    results.insert(
        "a".to_string(),
        ModelPoll {
            slots: Err(MonitorError::Timeout),
            metrics: Err(MonitorError::Timeout),
        },
    );

    apply_model_results(&mut snapshot, &loaded, &results);

    assert_eq!(
        snapshot.slots.get("a"),
        Some(&prior),
        "a failed poll must retain the last-known slots"
    );
    assert!(snapshot.metrics.contains_key("a"));
    assert!(snapshot.errors.iter().any(|e| e.slice == SLICE_SLOTS));
    assert!(snapshot.errors.iter().any(|e| e.slice == SLICE_METRICS));
}

#[test]
fn apply_model_results_with_no_loaded_models_preserves_existing_errors() {
    let mut snapshot = MonitorSnapshot::default();
    snapshot.set_slice_error(SLICE_SLOTS, "previous failure".to_string());

    apply_model_results(&mut snapshot, &[], &BTreeMap::new());

    assert!(
        snapshot.errors.iter().any(|e| e.slice == SLICE_SLOTS),
        "polling no models must not clear a recorded failure"
    );
}

#[test]
fn apply_hardware_result_failure_keeps_previous_gpus() {
    let mut snapshot = MonitorSnapshot {
        gpus: vec![GpuStats {
            index: 0,
            name: "GPU".to_string(),
            utilization_percent: Some(50.0),
            memory_total_mib: None,
            memory_used_mib: None,
            temperature_c: None,
            power_draw_w: None,
            power_limit_w: None,
        }],
        ..MonitorSnapshot::default()
    };

    apply_hardware_result(
        &mut snapshot,
        Err(MonitorError::Gpu("down".to_string())),
        true,
    );

    assert_eq!(snapshot.gpus.len(), 1);
    assert!(snapshot.gpu_available);
    assert!(snapshot.errors.iter().any(|e| e.slice == SLICE_HARDWARE));
}
