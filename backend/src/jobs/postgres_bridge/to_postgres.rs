use anyhow::Context;
use sqlx::{PgPool, Row, SqlitePool};

use crate::ApiError;

/// Controls which SQLite outputs should overwrite Postgres tables.
pub(super) struct OutputSyncOptions {
    pub(super) sync_charging_sessions: bool,
    pub(super) sync_charging_kpi_snapshots: bool,
    pub(super) sync_charging_rankings: bool,
    pub(super) sync_composite_kpi_snapshots: bool,
    pub(super) sync_composite_rankings: bool,
}

macro_rules! get_col {
    ($row:expr, $ty:ty, $column:literal) => {
        $row.try_get::<$ty, _>($column).with_context(|| {
            format!(
                "failed to read sqlite column `{}` while syncing job outputs",
                $column
            )
        })?
    };
}

/// Synchronizes SQLite-computed KPI/ranking outputs back into PostgreSQL tables.
pub(super) async fn sync_job_outputs_to_postgres(
    sqlite_pool: &SqlitePool,
    pg_pool: &PgPool,
    options: &OutputSyncOptions,
) -> Result<(), ApiError> {
    let mut pg_tx = pg_pool
        .begin()
        .await
        .context("failed to open postgres output sync transaction")?;

    clear_postgres_output_tables(&mut pg_tx, options).await?;
    if options.sync_charging_sessions {
        export_charging_sessions(sqlite_pool, &mut pg_tx).await?;
    }
    export_kpi_snapshots(sqlite_pool, &mut pg_tx, options).await?;
    export_ranking_snapshots(sqlite_pool, &mut pg_tx, options).await?;

    pg_tx
        .commit()
        .await
        .context("failed to commit postgres output sync transaction")?;
    Ok(())
}

async fn clear_postgres_output_tables(
    pg_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    options: &OutputSyncOptions,
) -> Result<(), ApiError> {
    let mut statements = Vec::new();
    if options.sync_charging_rankings && options.sync_composite_rankings {
        statements.push("DELETE FROM cohort_ranking_snapshot");
    } else if options.sync_charging_rankings {
        statements.push("DELETE FROM cohort_ranking_snapshot WHERE ranking_type <> 'ev_composite'");
    } else if options.sync_composite_rankings {
        statements.push(
            "DELETE FROM cohort_ranking_snapshot WHERE ranking_type <> 'ev_charging_performance'",
        );
    } else {
        statements.push(
            "DELETE FROM cohort_ranking_snapshot WHERE ranking_type NOT IN ('ev_charging_performance', 'ev_composite')",
        );
    }
    if options.sync_charging_kpi_snapshots && options.sync_composite_kpi_snapshots {
        statements.push("DELETE FROM vehicle_kpi_snapshot");
    } else if options.sync_charging_kpi_snapshots {
        statements.push("DELETE FROM vehicle_kpi_snapshot WHERE ranking_type <> 'ev_composite'");
    } else if options.sync_composite_kpi_snapshots {
        statements.push(
            "DELETE FROM vehicle_kpi_snapshot WHERE ranking_type <> 'ev_charging_performance'",
        );
    } else {
        statements.push(
            "DELETE FROM vehicle_kpi_snapshot WHERE ranking_type NOT IN ('ev_charging_performance', 'ev_composite')",
        );
    }
    if options.sync_charging_sessions {
        statements.push("DELETE FROM vehicle_charging_session");
    }

    for sql in statements {
        sqlx::query(sql)
            .execute(&mut **pg_tx)
            .await
            .with_context(|| format!("failed to clear postgres table with statement: {}", sql))?;
    }

    Ok(())
}

async fn export_charging_sessions(
    sqlite_pool: &SqlitePool,
    pg_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
          charging_session_id,
          vehicle_uid,
          session_id,
          started_at,
          ended_at,
          status,
          charger_type,
          soc_start_pct,
          soc_end_pct,
          soc_delta_pct,
          energy_added_kwh,
          avg_charge_power_kw,
          peak_charge_power_kw,
          ambient_temp_avg_c,
          battery_temp_avg_c,
          temperature_bin,
          temperature_is_estimated,
          sample_count,
          created_at,
          updated_at
        FROM vehicle_charging_session
        "#,
    )
    .fetch_all(sqlite_pool)
    .await
    .context("failed to fetch sqlite charging sessions for postgres sync")?;

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO vehicle_charging_session (
                charging_session_id,
                vehicle_uid,
                session_id,
                started_at,
                ended_at,
                status,
                charger_type,
                soc_start_pct,
                soc_end_pct,
                soc_delta_pct,
                energy_added_kwh,
                avg_charge_power_kw,
                peak_charge_power_kw,
                ambient_temp_avg_c,
                battery_temp_avg_c,
                temperature_bin,
                temperature_is_estimated,
                sample_count,
                created_at,
                updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
            )
            "#,
        )
        .bind(get_col!(row, String, "charging_session_id"))
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, String, "session_id"))
        .bind(get_col!(row, String, "started_at"))
        .bind(get_col!(row, Option<String>, "ended_at"))
        .bind(get_col!(row, String, "status"))
        .bind(get_col!(row, String, "charger_type"))
        .bind(get_col!(row, Option<f64>, "soc_start_pct"))
        .bind(get_col!(row, Option<f64>, "soc_end_pct"))
        .bind(get_col!(row, Option<f64>, "soc_delta_pct"))
        .bind(get_col!(row, Option<f64>, "energy_added_kwh"))
        .bind(get_col!(row, Option<f64>, "avg_charge_power_kw"))
        .bind(get_col!(row, Option<f64>, "peak_charge_power_kw"))
        .bind(get_col!(row, Option<f64>, "ambient_temp_avg_c"))
        .bind(get_col!(row, Option<f64>, "battery_temp_avg_c"))
        .bind(get_col!(row, Option<String>, "temperature_bin"))
        .bind(get_col!(row, i64, "temperature_is_estimated"))
        .bind(get_col!(row, i64, "sample_count"))
        .bind(get_col!(row, String, "created_at"))
        .bind(get_col!(row, String, "updated_at"))
        .execute(&mut **pg_tx)
        .await
        .context("failed to insert postgres charging session from sqlite output")?;
    }

    Ok(())
}

async fn export_kpi_snapshots(
    sqlite_pool: &SqlitePool,
    pg_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    options: &OutputSyncOptions,
) -> Result<(), ApiError> {
    let rows = if options.sync_charging_kpi_snapshots && options.sync_composite_kpi_snapshots {
        sqlx::query(
            r#"
            SELECT
              snapshot_id,
              vehicle_uid,
              ranking_type,
              timeframe,
              kpi_key,
              kpi_value,
              kpi_unit,
              direction,
              confidence_level,
              sample_count,
              temperature_bin,
              baseline_temperature_bin,
              compare_temperature_bin,
              computed_at,
              valid_from,
              valid_to,
              source_job_id
            FROM vehicle_kpi_snapshot
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context("failed to fetch sqlite KPI snapshots for postgres sync")?
    } else if options.sync_charging_kpi_snapshots {
        sqlx::query(
            r#"
            SELECT
              snapshot_id,
              vehicle_uid,
              ranking_type,
              timeframe,
              kpi_key,
              kpi_value,
              kpi_unit,
              direction,
              confidence_level,
              sample_count,
              temperature_bin,
              baseline_temperature_bin,
              compare_temperature_bin,
              computed_at,
              valid_from,
              valid_to,
              source_job_id
            FROM vehicle_kpi_snapshot
            WHERE ranking_type <> 'ev_composite'
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context("failed to fetch sqlite non-composite KPI snapshots for postgres sync")?
    } else if options.sync_composite_kpi_snapshots {
        sqlx::query(
            r#"
            SELECT
              snapshot_id,
              vehicle_uid,
              ranking_type,
              timeframe,
              kpi_key,
              kpi_value,
              kpi_unit,
              direction,
              confidence_level,
              sample_count,
              temperature_bin,
              baseline_temperature_bin,
              compare_temperature_bin,
              computed_at,
              valid_from,
              valid_to,
              source_job_id
            FROM vehicle_kpi_snapshot
            WHERE ranking_type <> 'ev_charging_performance'
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context("failed to fetch sqlite non-charging KPI snapshots for postgres sync")?
    } else {
        sqlx::query(
            r#"
            SELECT
              snapshot_id,
              vehicle_uid,
              ranking_type,
              timeframe,
              kpi_key,
              kpi_value,
              kpi_unit,
              direction,
              confidence_level,
              sample_count,
              temperature_bin,
              baseline_temperature_bin,
              compare_temperature_bin,
              computed_at,
              valid_from,
              valid_to,
              source_job_id
            FROM vehicle_kpi_snapshot
            WHERE ranking_type NOT IN ('ev_charging_performance', 'ev_composite')
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context(
            "failed to fetch sqlite non-charging/non-composite KPI snapshots for postgres sync",
        )?
    };

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO vehicle_kpi_snapshot (
                snapshot_id,
                vehicle_uid,
                ranking_type,
                timeframe,
                kpi_key,
                kpi_value,
                kpi_unit,
                direction,
                confidence_level,
                sample_count,
                temperature_bin,
                baseline_temperature_bin,
                compare_temperature_bin,
                computed_at,
                valid_from,
                valid_to,
                source_job_id
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16, $17
            )
            "#,
        )
        .bind(get_col!(row, String, "snapshot_id"))
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, String, "ranking_type"))
        .bind(get_col!(row, String, "timeframe"))
        .bind(get_col!(row, String, "kpi_key"))
        .bind(get_col!(row, f64, "kpi_value"))
        .bind(get_col!(row, Option<String>, "kpi_unit"))
        .bind(get_col!(row, String, "direction"))
        .bind(get_col!(row, String, "confidence_level"))
        .bind(get_col!(row, i64, "sample_count"))
        .bind(get_col!(row, String, "temperature_bin"))
        .bind(get_col!(row, Option<String>, "baseline_temperature_bin"))
        .bind(get_col!(row, Option<String>, "compare_temperature_bin"))
        .bind(get_col!(row, String, "computed_at"))
        .bind(get_col!(row, Option<String>, "valid_from"))
        .bind(get_col!(row, Option<String>, "valid_to"))
        .bind(get_col!(row, Option<String>, "source_job_id"))
        .execute(&mut **pg_tx)
        .await
        .context("failed to insert postgres KPI snapshot from sqlite output")?;
    }

    Ok(())
}

async fn export_ranking_snapshots(
    sqlite_pool: &SqlitePool,
    pg_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    options: &OutputSyncOptions,
) -> Result<(), ApiError> {
    let rows = if options.sync_charging_rankings && options.sync_composite_rankings {
        sqlx::query(
            r#"
            SELECT
              ranking_snapshot_id,
              ranking_type,
              timeframe,
              temperature_bin,
              cohort_key,
              cohort_size,
              sample_gate_passed,
              vehicle_uid,
              rank_position,
              score,
              confidence_level,
              computed_at
            FROM cohort_ranking_snapshot
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context("failed to fetch sqlite ranking snapshots for postgres sync")?
    } else if options.sync_charging_rankings {
        sqlx::query(
            r#"
            SELECT
              ranking_snapshot_id,
              ranking_type,
              timeframe,
              temperature_bin,
              cohort_key,
              cohort_size,
              sample_gate_passed,
              vehicle_uid,
              rank_position,
              score,
              confidence_level,
              computed_at
            FROM cohort_ranking_snapshot
            WHERE ranking_type <> 'ev_composite'
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context("failed to fetch sqlite non-composite ranking snapshots for postgres sync")?
    } else if options.sync_composite_rankings {
        sqlx::query(
            r#"
            SELECT
              ranking_snapshot_id,
              ranking_type,
              timeframe,
              temperature_bin,
              cohort_key,
              cohort_size,
              sample_gate_passed,
              vehicle_uid,
              rank_position,
              score,
              confidence_level,
              computed_at
            FROM cohort_ranking_snapshot
            WHERE ranking_type <> 'ev_charging_performance'
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context("failed to fetch sqlite non-charging ranking snapshots for postgres sync")?
    } else {
        sqlx::query(
            r#"
            SELECT
              ranking_snapshot_id,
              ranking_type,
              timeframe,
              temperature_bin,
              cohort_key,
              cohort_size,
              sample_gate_passed,
              vehicle_uid,
              rank_position,
              score,
              confidence_level,
              computed_at
            FROM cohort_ranking_snapshot
            WHERE ranking_type NOT IN ('ev_charging_performance', 'ev_composite')
            "#,
        )
        .fetch_all(sqlite_pool)
        .await
        .context(
            "failed to fetch sqlite non-charging/non-composite ranking snapshots for postgres sync",
        )?
    };

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO cohort_ranking_snapshot (
                ranking_snapshot_id,
                ranking_type,
                timeframe,
                temperature_bin,
                cohort_key,
                cohort_size,
                sample_gate_passed,
                vehicle_uid,
                rank_position,
                score,
                confidence_level,
                computed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(get_col!(row, String, "ranking_snapshot_id"))
        .bind(get_col!(row, String, "ranking_type"))
        .bind(get_col!(row, String, "timeframe"))
        .bind(get_col!(row, String, "temperature_bin"))
        .bind(get_col!(row, String, "cohort_key"))
        .bind(get_col!(row, i64, "cohort_size"))
        .bind(get_col!(row, i64, "sample_gate_passed"))
        .bind(get_col!(row, String, "vehicle_uid"))
        .bind(get_col!(row, i64, "rank_position"))
        .bind(get_col!(row, f64, "score"))
        .bind(get_col!(row, String, "confidence_level"))
        .bind(get_col!(row, String, "computed_at"))
        .execute(&mut **pg_tx)
        .await
        .context("failed to insert postgres ranking snapshot from sqlite output")?;
    }

    Ok(())
}
