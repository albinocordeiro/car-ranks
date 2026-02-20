use anyhow::Context;
use sqlx::{PgPool, Row, SqlitePool};

use crate::ApiError;

macro_rules! get_col {
    ($row:expr, $ty:ty, $column:literal) => {
        $row.try_get::<$ty, _>($column).with_context(|| {
            format!(
                "failed to read postgres column `{}` while syncing job inputs",
                $column
            )
        })?
    };
}

/// Synchronizes PostgreSQL raw/input tables into SQLite for KPI computation.
pub(super) async fn sync_job_inputs_from_postgres(
    pg_pool: &PgPool,
    sqlite_pool: &SqlitePool,
) -> Result<(), ApiError> {
    let mut sqlite_tx = sqlite_pool
        .begin()
        .await
        .context("failed to open sqlite sync transaction")?;

    clear_sqlite_job_tables(&mut sqlite_tx).await?;
    import_vehicle_rows(pg_pool, &mut sqlite_tx).await?;
    import_ingest_batch_rows(pg_pool, &mut sqlite_tx).await?;
    import_observation_rows(pg_pool, &mut sqlite_tx).await?;
    import_session_event_rows(pg_pool, &mut sqlite_tx).await?;
    import_diagnostic_event_rows(pg_pool, &mut sqlite_tx).await?;

    sqlite_tx
        .commit()
        .await
        .context("failed to commit sqlite input sync transaction")?;
    Ok(())
}

async fn clear_sqlite_job_tables(
    sqlite_tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ApiError> {
    for sql in [
        "DELETE FROM cohort_ranking_snapshot",
        "DELETE FROM vehicle_kpi_snapshot",
        "DELETE FROM vehicle_charging_session",
        "DELETE FROM vehicle_diagnostic_event",
        "DELETE FROM vehicle_session_event",
        "DELETE FROM vehicle_signal_observation",
        "DELETE FROM ingest_batch",
        "DELETE FROM user_vehicle_access",
        "DELETE FROM app_user",
        "DELETE FROM vehicle",
    ] {
        sqlx::query(sql)
            .execute(&mut **sqlite_tx)
            .await
            .with_context(|| format!("failed to clear sqlite table with statement: {}", sql))?;
    }

    Ok(())
}

async fn import_vehicle_rows(
    pg_pool: &PgPool,
    sqlite_tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
          vehicle_uid,
          source_account_id,
          vin_hash,
          make,
          model,
          trim,
          model_year,
          powertrain_class,
          created_at,
          updated_at
        FROM vehicle
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .context("failed to fetch postgres vehicle rows for sync")?;

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO vehicle (
                vehicle_uid,
                source_account_id,
                vin_hash,
                make,
                model,
                trim,
                model_year,
                powertrain_class,
                created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, String, "source_account_id"))
        .bind(get_col!(row, Option<String>, "vin_hash"))
        .bind(get_col!(row, Option<String>, "make"))
        .bind(get_col!(row, Option<String>, "model"))
        .bind(get_col!(row, Option<String>, "trim"))
        .bind(get_col!(row, Option<i64>, "model_year"))
        .bind(get_col!(row, String, "powertrain_class"))
        .bind(get_col!(row, String, "created_at"))
        .bind(get_col!(row, String, "updated_at"))
        .execute(&mut **sqlite_tx)
        .await
        .context("failed to insert synced sqlite vehicle row")?;
    }

    Ok(())
}

async fn import_ingest_batch_rows(
    pg_pool: &PgPool,
    sqlite_tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
          batch_id,
          vehicle_uid,
          schema_version,
          source,
          capture_started_at,
          capture_ended_at,
          received_at
        FROM ingest_batch
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .context("failed to fetch postgres ingest_batch rows for sync")?;

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO ingest_batch (
                batch_id,
                vehicle_uid,
                schema_version,
                source,
                capture_started_at,
                capture_ended_at,
                received_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(get_col!(row, String, "batch_id"))
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, String, "schema_version"))
        .bind(get_col!(row, String, "source"))
        .bind(get_col!(row, String, "capture_started_at"))
        .bind(get_col!(row, String, "capture_ended_at"))
        .bind(get_col!(row, String, "received_at"))
        .execute(&mut **sqlite_tx)
        .await
        .context("failed to insert synced sqlite ingest_batch row")?;
    }

    Ok(())
}

async fn import_observation_rows(
    pg_pool: &PgPool,
    sqlite_tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
          observation_id,
          vehicle_uid,
          batch_id,
          session_id,
          signal_key,
          value_number,
          value_string,
          value_bool,
          value_json,
          unit,
          observed_at,
          ingested_at,
          source,
          source_signal,
          status,
          confidence,
          freshness_ttl_seconds,
          temperature_bin,
          is_temperature_estimated,
          raw_payload_ref
        FROM vehicle_signal_observation
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .context("failed to fetch postgres observation rows for sync")?;

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO vehicle_signal_observation (
                observation_id,
                vehicle_uid,
                batch_id,
                session_id,
                signal_key,
                value_number,
                value_string,
                value_bool,
                value_json,
                unit,
                observed_at,
                ingested_at,
                source,
                source_signal,
                status,
                confidence,
                freshness_ttl_seconds,
                temperature_bin,
                is_temperature_estimated,
                raw_payload_ref
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )
            "#,
        )
        .bind(get_col!(row, String, "observation_id"))
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, Option<String>, "batch_id"))
        .bind(get_col!(row, Option<String>, "session_id"))
        .bind(get_col!(row, String, "signal_key"))
        .bind(get_col!(row, Option<f64>, "value_number"))
        .bind(get_col!(row, Option<String>, "value_string"))
        .bind(get_col!(row, Option<i64>, "value_bool"))
        .bind(get_col!(row, Option<String>, "value_json"))
        .bind(get_col!(row, Option<String>, "unit"))
        .bind(get_col!(row, String, "observed_at"))
        .bind(get_col!(row, String, "ingested_at"))
        .bind(get_col!(row, String, "source"))
        .bind(get_col!(row, Option<String>, "source_signal"))
        .bind(get_col!(row, String, "status"))
        .bind(get_col!(row, Option<f64>, "confidence"))
        .bind(get_col!(row, Option<i64>, "freshness_ttl_seconds"))
        .bind(get_col!(row, Option<String>, "temperature_bin"))
        .bind(get_col!(row, i64, "is_temperature_estimated"))
        .bind(get_col!(row, Option<String>, "raw_payload_ref"))
        .execute(&mut **sqlite_tx)
        .await
        .context("failed to insert synced sqlite observation row")?;
    }

    Ok(())
}

async fn import_session_event_rows(
    pg_pool: &PgPool,
    sqlite_tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
          session_event_id,
          vehicle_uid,
          session_id,
          session_type,
          event_type,
          observed_at,
          ingested_at,
          source,
          raw_payload_ref
        FROM vehicle_session_event
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .context("failed to fetch postgres session_event rows for sync")?;

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO vehicle_session_event (
                session_event_id,
                vehicle_uid,
                session_id,
                session_type,
                event_type,
                observed_at,
                ingested_at,
                source,
                raw_payload_ref
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(get_col!(row, String, "session_event_id"))
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, String, "session_id"))
        .bind(get_col!(row, String, "session_type"))
        .bind(get_col!(row, String, "event_type"))
        .bind(get_col!(row, String, "observed_at"))
        .bind(get_col!(row, String, "ingested_at"))
        .bind(get_col!(row, String, "source"))
        .bind(get_col!(row, Option<String>, "raw_payload_ref"))
        .execute(&mut **sqlite_tx)
        .await
        .context("failed to insert synced sqlite session_event row")?;
    }

    Ok(())
}

async fn import_diagnostic_event_rows(
    pg_pool: &PgPool,
    sqlite_tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<(), ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
          event_id,
          vehicle_uid,
          batch_id,
          session_id,
          event_type,
          code,
          severity,
          description,
          observed_at,
          ingested_at,
          source,
          source_event,
          resolution_hint
        FROM vehicle_diagnostic_event
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .context("failed to fetch postgres diagnostic rows for sync")?;

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO vehicle_diagnostic_event (
                event_id,
                vehicle_uid,
                batch_id,
                session_id,
                event_type,
                code,
                severity,
                description,
                observed_at,
                ingested_at,
                source,
                source_event,
                resolution_hint
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(get_col!(row, String, "event_id"))
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, Option<String>, "batch_id"))
        .bind(get_col!(row, Option<String>, "session_id"))
        .bind(get_col!(row, String, "event_type"))
        .bind(get_col!(row, Option<String>, "code"))
        .bind(get_col!(row, Option<String>, "severity"))
        .bind(get_col!(row, Option<String>, "description"))
        .bind(get_col!(row, String, "observed_at"))
        .bind(get_col!(row, String, "ingested_at"))
        .bind(get_col!(row, String, "source"))
        .bind(get_col!(row, Option<String>, "source_event"))
        .bind(get_col!(row, Option<String>, "resolution_hint"))
        .execute(&mut **sqlite_tx)
        .await
        .context("failed to insert synced sqlite diagnostic row")?;
    }

    Ok(())
}
