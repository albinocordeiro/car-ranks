use super::*;
use std::collections::BTreeMap;

use anyhow::{Context, Result};
use sqlx::sqlite::SqlitePoolOptions;

#[test]
fn temperature_bin_boundaries() {
    assert_eq!(derive_temperature_bin(-10.0), "very_cold");
    assert_eq!(derive_temperature_bin(-5.0), "very_cold");
    assert_eq!(derive_temperature_bin(-4.9), "cold");
    assert_eq!(derive_temperature_bin(5.0), "cold");
    assert_eq!(derive_temperature_bin(10.0), "cool");
    assert_eq!(derive_temperature_bin(20.0), "mild");
    assert_eq!(derive_temperature_bin(30.0), "hot");
}

#[test]
fn percentile_higher_is_better() {
    let values = vec![50.0, 60.0, 70.0, 80.0];
    assert_eq!(percentile_rank(&values, 70.0, true), 75);
    assert_eq!(percentile_rank(&values, 50.0, true), 25);
}

#[test]
fn percentile_lower_is_better() {
    let values = vec![10.0, 20.0, 30.0, 40.0];
    assert_eq!(percentile_rank(&values, 20.0, false), 75);
    assert_eq!(percentile_rank(&values, 40.0, false), 25);
}

#[test]
fn locked_kpi_catalog_contains_core_composite_metric() {
    let spec = kpi_specs::lookup_kpi_spec("ev_composite", "ev_composite_score");
    assert!(spec.is_some());
}

#[test]
fn wh_per_km_from_soc_delta_works() {
    let wh_per_km = metrics::wh_per_km_from_soc_delta(5.0, 20.0, 60.0).expect("expected value");
    assert!((wh_per_km - 150.0).abs() < 0.0001);
}

#[test]
fn score_from_kpi_map_range_fallback_uses_net_efficiency() {
    let mut kpis = BTreeMap::new();
    kpis.insert("ev_estimated_practical_range".to_string(), 280.0);
    kpis.insert("ev_net_energy_efficiency".to_string(), 160.0);

    let score = metrics::score_from_kpi_map("ev_range_efficiency", &kpis);
    assert!(score > 0.0);
    assert!(score <= 100.0);
}

#[test]
fn sqlite_migration_matches_legacy_schema_snapshot() {
    assert_eq!(SQLITE_MIGRATION_0001, LEGACY_SQLITE_SCHEMA);
}

#[test]
fn postgres_migration_has_expected_base_tables() {
    assert!(!POSTGRES_MIGRATION_0001.contains("PRAGMA"));
    for table_name in [
        "vehicle",
        "ingest_batch",
        "vehicle_signal_observation",
        "vehicle_diagnostic_event",
        "vehicle_session_event",
        "vehicle_charging_session",
        "vehicle_kpi_snapshot",
        "cohort_ranking_snapshot",
    ] {
        let marker = format!("CREATE TABLE IF NOT EXISTS {}", table_name);
        assert!(
            POSTGRES_MIGRATION_0001.contains(&marker),
            "missing table in postgres migration: {}",
            table_name
        );
    }
}

#[tokio::test]
async fn apply_schema_records_migrations_once() -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;

    apply_schema(&pool).await?;
    apply_schema(&pool).await?;

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM schema_migration
        WHERE migration_id = '0001_init'
          AND backend = 'sqlite'
        "#,
    )
    .fetch_one(&pool)
    .await
    .context("failed to count applied migrations")?;

    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn temperature_sample_gate_checks() {
    let gates = metrics::TemperatureSampleGates {
        min_cold_distance_km: 20.0,
        min_mild_distance_km: 20.0,
        min_cold_charge_sessions: 1,
        min_mild_charge_sessions: 1,
        min_sensitivity_points: 6,
    };

    assert!(gates.range_gate_passed(20.0, 25.0));
    assert!(!gates.range_gate_passed(19.9, 25.0));
    assert!(!gates.range_gate_passed(20.0, 19.9));

    assert!(gates.charge_gate_passed(1, 1));
    assert!(!gates.charge_gate_passed(0, 1));
    assert!(!gates.charge_gate_passed(1, 0));
}

mod kpi_job;

mod ingest_paths;
mod postgres_paths;
