use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, TelemetryBatchRequest};

/// Persists diagnostic events (MIL state changes and active DTC snapshots).
pub(in crate::ingest) async fn insert_diagnostic_events(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    for diag in &payload.diagnostics {
        if let Some(mil_on) = diag.mil_on {
            sqlx::query(
                r#"
                INSERT INTO vehicle_diagnostic_event (
                    event_id,
                    vehicle_uid,
                    batch_id,
                    event_type,
                    observed_at,
                    ingested_at,
                    source
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(vehicle_uid)
            .bind(payload.batch_id.to_string())
            .bind(if mil_on { "MIL_ON" } else { "MIL_OFF" })
            .bind(diag.observed_at.to_rfc3339())
            .bind(now)
            .bind("OBD")
            .execute(&mut **tx)
            .await
            .context("failed to insert MIL diagnostic event")?;
        }

        if let Some(dtcs) = &diag.dtcs_active {
            for code in dtcs {
                sqlx::query(
                    r#"
                    INSERT INTO vehicle_diagnostic_event (
                        event_id,
                        vehicle_uid,
                        batch_id,
                        event_type,
                        code,
                        observed_at,
                        ingested_at,
                        source
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(vehicle_uid)
                .bind(payload.batch_id.to_string())
                .bind("DTC_ACTIVE")
                .bind(code)
                .bind(diag.observed_at.to_rfc3339())
                .bind(now)
                .bind("OBD")
                .execute(&mut **tx)
                .await
                .context("failed to insert DTC diagnostic event")?;
            }
        }
    }

    Ok(())
}
