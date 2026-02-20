use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use super::*;

async fn test_app_state() -> Result<AppState> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;
    apply_schema(&pool).await?;
    Ok(AppState {
        sqlite_pool: pool,
        pg_pool: None,
        backend: DatabaseBackend::Sqlite,
        signal_keys: Arc::new(load_signal_keys()?),
    })
}

fn valid_ingest_payload(
    vehicle_uid: Uuid,
    batch_id: Uuid,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
) -> TelemetryBatchRequest {
    TelemetryBatchRequest {
        batch_id,
        schema_version: INGEST_SCHEMA_VERSION.to_string(),
        vehicle_uid,
        source: "OBD".to_string(),
        client: Some(ClientInfo {
            platform: Some("ios".to_string()),
            app_version: Some("1.0.0-test".to_string()),
            adapter_fingerprint: Some("adapter-test".to_string()),
        }),
        capture_window: CaptureWindow {
            started_at,
            ended_at,
            sample_interval_seconds: Some(60),
        },
        records: vec![TelemetryRecord {
            observed_at: started_at + Duration::seconds(5),
            signal_key: "speed.vehicle".to_string(),
            value_number: Some(42.0),
            value_string: None,
            value_bool: None,
            value_json: None,
            unit: Some("km/h".to_string()),
            status: "ok".to_string(),
            confidence: Some(0.95),
            source_signal: Some("01_0D".to_string()),
            freshness_ttl_seconds: Some(30),
            temperature_bin: None,
            is_temperature_estimated: Some(false),
            session_id: None,
            raw_payload_ref: None,
        }],
        session_events: Vec::new(),
        diagnostics: Vec::new(),
    }
}

#[tokio::test]
async fn ingest_duplicate_same_envelope_returns_duplicate_true() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );

    let Json(first_response) = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .map_err(|err| anyhow::anyhow!("first ingest failed: {} {}", err.error, err.message))?;
    assert!(first_response.accepted);
    assert!(!first_response.duplicate);

    let duplicate_payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    let Json(duplicate_response) =
        post_telemetry_batches(State(state.clone()), Json(duplicate_payload))
            .await
            .map_err(|err| {
                anyhow::anyhow!("duplicate ingest failed: {} {}", err.error, err.message)
            })?;
    assert!(duplicate_response.accepted);
    assert!(duplicate_response.duplicate);
    assert_eq!(duplicate_response.records_accepted, 0);
    Ok(())
}

#[tokio::test]
async fn ingest_duplicate_with_different_envelope_returns_conflict() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    let _ = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .map_err(|err| anyhow::anyhow!("first ingest failed: {} {}", err.error, err.message))?;

    let conflict_payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::seconds(10),
    );
    let err = post_telemetry_batches(State(state.clone()), Json(conflict_payload))
        .await
        .expect_err("expected idempotency conflict");

    assert_eq!(err.status, StatusCode::CONFLICT);
    assert_eq!(err.error, "conflict");
    Ok(())
}

#[tokio::test]
async fn ingest_rejects_unsupported_schema_version() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let mut payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    payload.schema_version = "1.0".to_string();

    let err = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .expect_err("expected schema_version rejection");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.error, "bad_request");
    Ok(())
}

#[tokio::test]
async fn ingest_rejects_record_outside_capture_window() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let mut payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    payload.records[0].observed_at = now;

    let err = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .expect_err("expected out-of-window rejection");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.error, "bad_request");
    Ok(())
}

#[tokio::test]
async fn ingest_rejects_unknown_session_event_type() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let mut payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    payload.session_events.push(SessionEventInput {
        event_type: "invalid_session_event".to_string(),
        observed_at: now - Duration::minutes(1) + Duration::seconds(10),
        session_id: Uuid::new_v4(),
    });

    let err = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .expect_err("expected session event type rejection");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.error, "bad_request");
    Ok(())
}
