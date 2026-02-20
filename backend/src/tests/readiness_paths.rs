use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use chrono::{Duration, Utc};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use super::*;

fn auth_context(user_id: Uuid) -> crate::auth::AuthContext {
    crate::auth::AuthContext::from_user_id(user_id)
}

async fn readiness_test_state() -> Result<AppState> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;
    crate::migrations::apply_schema(&pool).await?;
    Ok(AppState {
        sqlite_pool: pool,
        pg_pool: None,
        backend: DatabaseBackend::Sqlite,
        signal_keys: Arc::new(load_signal_keys()?),
    })
}

async fn insert_vehicle_owner(
    state: &AppState,
    vehicle_uid: Uuid,
    user_id: Uuid,
    now_ts: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO vehicle (
          vehicle_uid,
          source_account_id,
          make,
          model,
          powertrain_class,
          created_at,
          updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(vehicle_uid.to_string())
    .bind("sqlite-test-account")
    .bind("test_make")
    .bind("test_model")
    .bind("bev")
    .bind(now_ts)
    .bind(now_ts)
    .execute(&state.sqlite_pool)
    .await
    .context("failed to insert sqlite readiness test vehicle")?;

    sqlx::query(
        r#"
        INSERT INTO app_user (
          user_id,
          created_at,
          updated_at
        ) VALUES (?, ?, ?)
        "#,
    )
    .bind(user_id.to_string())
    .bind(now_ts)
    .bind(now_ts)
    .execute(&state.sqlite_pool)
    .await
    .context("failed to insert sqlite readiness app user")?;

    sqlx::query(
        r#"
        INSERT INTO user_vehicle_access (
          user_id,
          vehicle_uid,
          access_role,
          created_at,
          updated_at
        ) VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(user_id.to_string())
    .bind(vehicle_uid.to_string())
    .bind("owner")
    .bind(now_ts)
    .bind(now_ts)
    .execute(&state.sqlite_pool)
    .await
    .context("failed to insert sqlite readiness ownership link")?;

    Ok(())
}

#[tokio::test]
async fn readiness_reports_family_status_and_temperature_gaps_for_sqlite() -> Result<()> {
    let state = readiness_test_state().await?;
    let auth_user_id = Uuid::new_v4();
    let vehicle_uid = Uuid::new_v4();
    let now = Utc::now();
    let now_ts = now.to_rfc3339();

    insert_vehicle_owner(&state, vehicle_uid, auth_user_id, &now_ts).await?;

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
          computed_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid.to_string())
    .bind("ev_range_efficiency")
    .bind("90d")
    .bind("ev_net_energy_efficiency")
    .bind(185.0_f64)
    .bind("wh_per_km")
    .bind("lower_is_better")
    .bind("preview")
    .bind(5_i64)
    .bind("all")
    .bind(&now_ts)
    .execute(&state.sqlite_pool)
    .await
    .context("failed to insert readiness range KPI snapshot")?;

    // Seed odometer windows so mild passes the default distance gate while cold fails.
    for (temperature_bin, start_odo, end_odo) in
        [("cold", 1000.0, 1008.0), ("mild", 2000.0, 2028.0)]
    {
        sqlx::query(
            r#"
            INSERT INTO vehicle_signal_observation (
              observation_id,
              vehicle_uid,
              signal_key,
              value_number,
              observed_at,
              ingested_at,
              source,
              status,
              temperature_bin,
              is_temperature_estimated
            ) VALUES (?, ?, 'distance.odometer', ?, ?, ?, 'OBD', 'ok', ?, 0)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(vehicle_uid.to_string())
        .bind(start_odo)
        .bind((now - Duration::minutes(30)).to_rfc3339())
        .bind(&now_ts)
        .bind(temperature_bin)
        .execute(&state.sqlite_pool)
        .await
        .context("failed to insert readiness odometer start point")?;

        sqlx::query(
            r#"
            INSERT INTO vehicle_signal_observation (
              observation_id,
              vehicle_uid,
              signal_key,
              value_number,
              observed_at,
              ingested_at,
              source,
              status,
              temperature_bin,
              is_temperature_estimated
            ) VALUES (?, ?, 'distance.odometer', ?, ?, ?, 'OBD', 'ok', ?, 0)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(vehicle_uid.to_string())
        .bind(end_odo)
        .bind((now - Duration::minutes(10)).to_rfc3339())
        .bind(&now_ts)
        .bind(temperature_bin)
        .execute(&state.sqlite_pool)
        .await
        .context("failed to insert readiness odometer end point")?;
    }

    let Json(response) = crate::handlers::get_kpis_readiness(
        State(state.clone()),
        auth_context(auth_user_id),
        Query(ReadinessQuery {
            vehicle_uid,
            timeframe: Some("90d".to_string()),
        }),
    )
    .await
    .map_err(|err| anyhow::anyhow!("readiness handler failed: {}", err.message))?;

    assert_eq!(response.families.len(), 4);

    let range_family = response
        .families
        .iter()
        .find(|family| family.ranking_type == "ev_range_efficiency")
        .context("range readiness family missing")?;
    assert_eq!(range_family.confidence_level, "preview");
    assert_eq!(range_family.status, "preview");
    assert_eq!(range_family.sample_count, 5);

    let temp_family = response
        .families
        .iter()
        .find(|family| family.ranking_type == "ev_temperature_impact")
        .context("temperature readiness family missing")?;
    assert_eq!(temp_family.status, "not_ready");
    assert!(
        temp_family
            .missing_requirements
            .iter()
            .any(|item| item.starts_with("cold_distance_km<"))
    );
    assert!(
        temp_family
            .missing_requirements
            .iter()
            .any(|item| item.starts_with("cold_charging_sessions<"))
    );
    assert!(
        temp_family
            .missing_requirements
            .iter()
            .any(|item| item.starts_with("mild_charging_sessions<"))
    );
    assert!(
        temp_family
            .missing_requirements
            .iter()
            .any(|item| item == "temperature_kpis_missing")
    );

    Ok(())
}

#[tokio::test]
async fn readiness_rejects_unsupported_timeframe() -> Result<()> {
    let state = readiness_test_state().await?;
    let auth_user_id = Uuid::new_v4();
    let vehicle_uid = Uuid::new_v4();
    let now_ts = Utc::now().to_rfc3339();
    insert_vehicle_owner(&state, vehicle_uid, auth_user_id, &now_ts).await?;

    let err = crate::handlers::get_kpis_readiness(
        State(state),
        auth_context(auth_user_id),
        Query(ReadinessQuery {
            vehicle_uid,
            timeframe: Some("365d".to_string()),
        }),
    )
    .await
    .expect_err("expected unsupported timeframe rejection");

    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err.error, "unprocessable_entity");
    assert!(err.message.contains("unsupported timeframe"));

    Ok(())
}
