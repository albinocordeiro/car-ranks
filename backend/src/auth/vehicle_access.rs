use anyhow::Context;
use uuid::Uuid;

use crate::{ApiError, AppState};

/// Ensures the caller is authorized to read data for a specific vehicle.
///
/// Access control is backed by `user_vehicle_access` in Postgres.
pub(crate) async fn ensure_vehicle_access(
    state: &AppState,
    user_id: Uuid,
    vehicle_uid: Uuid,
) -> Result<(), ApiError> {
    let user_id_str = user_id.to_string();
    let vehicle_uid_str = vehicle_uid.to_string();

    // Use `EXISTS` to keep decoding stable across SQL type differences.
    let has_access = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM user_vehicle_access
            WHERE user_id = $1
              AND vehicle_uid = $2
        )
        "#,
    )
    .bind(&user_id_str)
    .bind(&vehicle_uid_str)
    .fetch_one(&state.pg_pool)
    .await
    .context("failed to verify postgres vehicle access")?;

    if !has_access {
        return Err(ApiError::forbidden("vehicle access denied for this user"));
    }

    Ok(())
}
