use anyhow::{Context, Result};
use sqlx::SqlitePool;

use super::SQLITE_MIGRATIONS;

pub(crate) async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    // Track migration state explicitly so schema evolution can move to additive migrations.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migration (
          migration_id TEXT PRIMARY KEY,
          backend TEXT NOT NULL,
          applied_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .context("failed to ensure schema_migration table")?;

    for (migration_id, migration_sql) in SQLITE_MIGRATIONS {
        let already_applied = sqlx::query("SELECT 1 FROM schema_migration WHERE migration_id = ?")
            .bind(migration_id)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("failed to check migration {}", migration_id))?
            .is_some();
        if already_applied {
            continue;
        }

        // Apply each statement in order so failures can report the precise failing fragment.
        for statement in migration_sql.split(';') {
            let stmt = statement.trim();
            if stmt.is_empty() {
                continue;
            }
            let sql = format!("{};", stmt);
            sqlx::query(&sql).execute(pool).await.with_context(|| {
                format!(
                    "failed to apply migration {} statement: {}",
                    migration_id, stmt
                )
            })?;
        }

        sqlx::query(
            r#"
            INSERT INTO schema_migration (migration_id, backend, applied_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(migration_id)
        .bind("sqlite")
        .bind(crate::now_str())
        .execute(pool)
        .await
        .with_context(|| format!("failed to record migration {}", migration_id))?;
    }

    Ok(())
}
