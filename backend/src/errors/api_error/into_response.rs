use axum::Json;
use serde::Serialize;

use super::ApiError;

/// Serialized body contract returned for every `ApiError`.
#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    message: String,
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ErrorBody {
            error: self.error,
            message: self.message,
        });
        (self.status, body).into_response()
    }
}
