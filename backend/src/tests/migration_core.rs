use anyhow::{Context, Result};
use sqlx::sqlite::SqlitePoolOptions;

use super::*;

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
