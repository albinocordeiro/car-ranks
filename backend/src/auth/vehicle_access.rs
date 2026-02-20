use anyhow::Context;
use uuid::Uuid;

use crate::{ApiError, AppState, DatabaseBackend};

/// Ensures the caller is authorized to read data for a specific vehicle.
///
/// This check is intentionally backend-aware so the same auth contract works
/// for both SQLite and Postgres runtime modes.
pub(crate) async fn ensure_vehicle_access(
    state: &AppState,
    user_id: Uuid,
    vehicle_uid: Uuid,
) -> Result<(), ApiError> {
    let user_id_str = user_id.to_string();
    let vehicle_uid_str = vehicle_uid.to_string();

    let has_access = match state.backend {
        DatabaseBackend::Sqlite => sqlx::query_scalar::<_, i64>(
            r#"
                SELECT 1
                FROM user_vehicle_access
                WHERE user_id = ?
                  AND vehicle_uid = ?
                LIMIT 1
                "#,
        )
        .bind(&user_id_str)
        .bind(&vehicle_uid_str)
        .fetch_optional(&state.sqlite_pool)
        .await
        .context("failed to verify sqlite vehicle access")?
        .is_some(),
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres backend missing pool"))?;
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT 1
                FROM user_vehicle_access
                WHERE user_id = $1
                  AND vehicle_uid = $2
                LIMIT 1
                "#,
            )
            .bind(&user_id_str)
            .bind(&vehicle_uid_str)
            .fetch_optional(pg_pool)
            .await
            .context("failed to verify postgres vehicle access")?
            .is_some()
        }
    };

    if !has_access {
        return Err(ApiError::forbidden("vehicle access denied for this user"));
    }

    Ok(())
}
