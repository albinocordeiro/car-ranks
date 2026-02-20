use axum::http::StatusCode;

use super::ApiError;

impl ApiError {
    /// Builds a 400 response for request payloads that violate API contracts.
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "bad_request".to_string(),
            message: message.into(),
        }
    }

    /// Builds a 404 response when requested entities are not available.
    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: "not_found".to_string(),
            message: message.into(),
        }
    }

    /// Builds a 422 response for semantically invalid but well-formed requests.
    pub(crate) fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            error: "unprocessable_entity".to_string(),
            message: message.into(),
        }
    }

    /// Builds a 401 response when authentication context is missing or invalid.
    pub(crate) fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            error: "unauthorized".to_string(),
            message: message.into(),
        }
    }

    /// Builds a 403 response when caller identity is valid but access is denied.
    pub(crate) fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            error: "forbidden".to_string(),
            message: message.into(),
        }
    }

    /// Builds a 409 response when the request collides with existing state.
    pub(crate) fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            error: "conflict".to_string(),
            message: message.into(),
        }
    }

    /// Builds a 500 response for unexpected server-side failures.
    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: "internal_error".to_string(),
            message: message.into(),
        }
    }
}
