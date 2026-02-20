use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use super::{AuthContext, USER_ID_HEADER};
use crate::ApiError;

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let raw_user_id = parts
            .headers
            .get(USER_ID_HEADER)
            .ok_or_else(|| {
                ApiError::unauthorized(format!("missing required {} header", USER_ID_HEADER))
            })?
            .to_str()
            .map_err(|_| {
                ApiError::unauthorized(format!("{} must be valid UTF-8", USER_ID_HEADER))
            })?;

        let user_id = Uuid::parse_str(raw_user_id)
            .map_err(|_| ApiError::unauthorized(format!("{} must be a UUID", USER_ID_HEADER)))?;

        Ok(AuthContext::from_user_id(user_id))
    }
}
