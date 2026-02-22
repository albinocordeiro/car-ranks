use anyhow::Context;
use axum::extract::{Query, State};
use axum::Json;
use sqlx::postgres::PgRow;
use sqlx::Row;

use crate::{
    now_str, parse_ts, ApiError, AppState, RawTelemetryQuery, RawTelemetryRecord,
    RawTelemetryResponse,
};

struct RawCursor {
    observed_at: String,
    observation_id: String,
}

/// Fetches recently ingested raw signal rows for one vehicle.
pub(crate) async fn get_raw_telemetry(
    State(state): State<AppState>,
    Query(params): Query<RawTelemetryQuery>,
) -> Result<Json<RawTelemetryResponse>, ApiError> {
    let limit = params.limit.unwrap_or(120).clamp(1, 500);
    let include_session_events = params.include_session_events.unwrap_or(false);
    let normalized_signal_key = params
        .signal_key
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let vehicle_uid = params.vehicle_uid.to_string();
    let batch_id = params.batch_id.map(|value| value.to_string());
    let session_id = params.session_id.map(|value| value.to_string());
    let cursor = normalize_cursor(
        params.cursor_observed_at.as_deref(),
        params.cursor_observation_id.as_deref(),
    )?;

    let cursor_observed_at = cursor.as_ref().map(|value| value.observed_at.clone());
    let cursor_observation_id = cursor.as_ref().map(|value| value.observation_id.clone());

    let signal_rows = sqlx::query(
        r#"
        SELECT
            observation_id,
            batch_id,
            session_id,
            observed_at,
            signal_key,
            source_signal,
            status,
            value_number::double precision AS value_number,
            value_string,
            value_bool,
            value_json,
            raw_payload_ref
        FROM vehicle_signal_observation
        WHERE vehicle_uid = $1
          AND raw_payload_ref IS NOT NULL
          AND ($2::text IS NULL OR signal_key = $2)
          AND ($3::text IS NULL OR batch_id = $3)
          AND ($4::text IS NULL OR session_id = $4)
          AND (
            $5::timestamptz IS NULL
            OR observed_at::timestamptz < $5::timestamptz
            OR (
              observed_at::timestamptz = $5::timestamptz
              AND observation_id < COALESCE($6::text, '')
            )
          )
        ORDER BY observed_at DESC, observation_id DESC
        LIMIT $7
        "#,
    )
    .bind(&vehicle_uid)
    .bind(&normalized_signal_key)
    .bind(&batch_id)
    .bind(&session_id)
    .bind(&cursor_observed_at)
    .bind(&cursor_observation_id)
    .bind(limit)
    .fetch_all(&state.pg_pool)
    .await
    .context("failed to fetch postgres raw telemetry rows")?;

    let mut mapped_rows = signal_rows
        .into_iter()
        .map(map_row)
        .collect::<Result<Vec<_>, _>>()?;

    if include_session_events && batch_id.is_none() {
        let session_rows = sqlx::query(
            r#"
            SELECT
                session_event_id AS observation_id,
                NULL::text AS batch_id,
                session_id,
                observed_at,
                CONCAT('session.', session_type, '.', event_type) AS signal_key,
                NULL::text AS source_signal,
                event_type AS status,
                NULL::double precision AS value_number,
                NULL::text AS value_string,
                NULL::bigint AS value_bool,
                NULL::text AS value_json,
                raw_payload_ref
            FROM vehicle_session_event
            WHERE vehicle_uid = $1
              AND raw_payload_ref IS NOT NULL
              AND ($2::text IS NULL OR CONCAT('session.', session_type, '.', event_type) = $2)
              AND ($3::text IS NULL OR session_id = $3)
              AND (
                $4::timestamptz IS NULL
                OR observed_at::timestamptz < $4::timestamptz
                OR (
                  observed_at::timestamptz = $4::timestamptz
                  AND session_event_id < COALESCE($5::text, '')
                )
              )
            ORDER BY observed_at DESC, session_event_id DESC
            LIMIT $6
            "#,
        )
        .bind(&vehicle_uid)
        .bind(&normalized_signal_key)
        .bind(&session_id)
        .bind(&cursor_observed_at)
        .bind(&cursor_observation_id)
        .bind(limit)
        .fetch_all(&state.pg_pool)
        .await
        .context("failed to fetch postgres raw session-event telemetry rows")?;

        let mut mapped_session_rows = session_rows
            .into_iter()
            .map(map_row)
            .collect::<Result<Vec<_>, _>>()?;
        mapped_rows.append(&mut mapped_session_rows);
    }

    mapped_rows.sort_by(|left, right| {
        let left_ts = parse_ts(&left.observed_at);
        let right_ts = parse_ts(&right.observed_at);
        right_ts
            .cmp(&left_ts)
            .then_with(|| right.observation_id.cmp(&left.observation_id))
    });
    mapped_rows.truncate(limit as usize);

    let returned_count = mapped_rows.len();
    let (next_cursor_observed_at, next_cursor_observation_id) = if returned_count == limit as usize
    {
        if let Some(last_row) = mapped_rows.last() {
            (
                Some(last_row.observed_at.clone()),
                Some(last_row.observation_id.clone()),
            )
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    Ok(Json(RawTelemetryResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        limit,
        signal_key: normalized_signal_key,
        batch_id: params.batch_id,
        session_id: params.session_id,
        include_session_events,
        cursor_observed_at,
        cursor_observation_id,
        next_cursor_observed_at,
        next_cursor_observation_id,
        returned_count,
        rows: mapped_rows,
    }))
}

fn normalize_cursor(
    cursor_observed_at: Option<&str>,
    cursor_observation_id: Option<&str>,
) -> Result<Option<RawCursor>, ApiError> {
    match (cursor_observed_at, cursor_observation_id) {
        (None, None) => Ok(None),
        (Some(observed_at), Some(observation_id)) => {
            let observed_at_trimmed = observed_at.trim();
            let observation_id_trimmed = observation_id.trim();
            if observed_at_trimmed.is_empty() || observation_id_trimmed.is_empty() {
                return Err(ApiError::bad_request(
                    "cursor_observed_at and cursor_observation_id must both be non-empty when provided",
                ));
            }
            if parse_ts(observed_at_trimmed).is_none() {
                return Err(ApiError::bad_request(
                    "cursor_observed_at must be a valid RFC3339 timestamp",
                ));
            }
            Ok(Some(RawCursor {
                observed_at: observed_at_trimmed.to_string(),
                observation_id: observation_id_trimmed.to_string(),
            }))
        }
        _ => Err(ApiError::bad_request(
            "cursor_observed_at and cursor_observation_id must be provided together",
        )),
    }
}

fn map_row(row: PgRow) -> Result<RawTelemetryRecord, ApiError> {
    let observation_id = row
        .try_get::<String, _>("observation_id")
        .context("failed to decode raw telemetry observation_id")?;
    let batch_id = row
        .try_get::<Option<String>, _>("batch_id")
        .context("failed to decode raw telemetry batch_id")?;
    let session_id = row
        .try_get::<Option<String>, _>("session_id")
        .context("failed to decode raw telemetry session_id")?;
    let observed_at = row
        .try_get::<String, _>("observed_at")
        .context("failed to decode raw telemetry observed_at")?;
    let signal_key = row
        .try_get::<String, _>("signal_key")
        .context("failed to decode raw telemetry signal_key")?;
    let source_signal = row
        .try_get::<Option<String>, _>("source_signal")
        .context("failed to decode raw telemetry source_signal")?;
    let status = row
        .try_get::<String, _>("status")
        .context("failed to decode raw telemetry status")?;
    let value_number = row
        .try_get::<Option<f64>, _>("value_number")
        .context("failed to decode raw telemetry value_number")?;
    let value_string = row
        .try_get::<Option<String>, _>("value_string")
        .context("failed to decode raw telemetry value_string")?;
    let value_bool = row
        .try_get::<Option<i64>, _>("value_bool")
        .context("failed to decode raw telemetry value_bool")?
        .map(|value| value != 0);
    let value_json = row
        .try_get::<Option<String>, _>("value_json")
        .context("failed to decode raw telemetry value_json")?;
    let raw_payload_ref = row
        .try_get::<Option<String>, _>("raw_payload_ref")
        .context("failed to decode raw telemetry raw_payload_ref")?;

    Ok(RawTelemetryRecord {
        observation_id,
        batch_id,
        session_id,
        observed_at,
        signal_key,
        source_signal,
        status,
        value_number,
        value_string,
        value_bool,
        value_json,
        raw_payload_ref,
    })
}
