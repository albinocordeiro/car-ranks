use anyhow::Context;
use sqlx::{Postgres, Sqlite, Transaction};

use crate::ApiError;

/// Binds a vehicle to its owner account during ingest.
///
/// MVP ownership policy is intentionally strict: one vehicle belongs to one
/// user account. A second user attempting to ingest for that vehicle is denied.
pub(crate) async fn bind_vehicle_owner_sqlite(
    tx: &mut Transaction<'_, Sqlite>,
    user_id: &str,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO app_user (
            user_id,
            created_at,
            updated_at
        ) VALUES (?, ?, ?)
        "#,
    )
    .bind(user_id)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to ensure app_user row")?;

    let existing_owner = sqlx::query_scalar::<_, String>(
        r#"
        SELECT user_id
        FROM user_vehicle_access
        WHERE vehicle_uid = ?
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .fetch_optional(&mut **tx)
    .await
    .context("failed to check existing vehicle owner")?;

    if let Some(existing_owner) = existing_owner {
        if existing_owner != user_id {
            return Err(ApiError::forbidden(
                "vehicle is already linked to a different user",
            ));
        }

        // Keep ownership metadata fresh for existing owner links.
        sqlx::query(
            r#"
            UPDATE user_vehicle_access
            SET updated_at = ?
            WHERE user_id = ?
              AND vehicle_uid = ?
            "#,
        )
        .bind(now)
        .bind(user_id)
        .bind(vehicle_uid)
        .execute(&mut **tx)
        .await
        .context("failed to update vehicle ownership timestamp")?;

        return Ok(());
    }

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
    .bind(user_id)
    .bind(vehicle_uid)
    .bind("owner")
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to insert vehicle ownership link")?;

    Ok(())
}

/// PostgreSQL variant of the ingest-time vehicle ownership binder.
pub(crate) async fn bind_vehicle_owner_postgres(
    tx: &mut Transaction<'_, Postgres>,
    user_id: &str,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO app_user (
            user_id,
            created_at,
            updated_at
        ) VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to ensure postgres app_user row")?;

    let existing_owner = sqlx::query_scalar::<_, String>(
        r#"
        SELECT user_id
        FROM user_vehicle_access
        WHERE vehicle_uid = $1
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .fetch_optional(&mut **tx)
    .await
    .context("failed to check existing postgres vehicle owner")?;

    if let Some(existing_owner) = existing_owner {
        if existing_owner != user_id {
            return Err(ApiError::forbidden(
                "vehicle is already linked to a different user",
            ));
        }

        sqlx::query(
            r#"
            UPDATE user_vehicle_access
            SET updated_at = $1
            WHERE user_id = $2
              AND vehicle_uid = $3
            "#,
        )
        .bind(now)
        .bind(user_id)
        .bind(vehicle_uid)
        .execute(&mut **tx)
        .await
        .context("failed to update postgres vehicle ownership timestamp")?;

        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO user_vehicle_access (
            user_id,
            vehicle_uid,
            access_role,
            created_at,
            updated_at
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id)
    .bind(vehicle_uid)
    .bind("owner")
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to insert postgres vehicle ownership link")?;

    Ok(())
}
