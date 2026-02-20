use axum::http::StatusCode;

mod constructors;
mod into_response;
mod source_conversion;

/// Canonical API error envelope used by handlers and domain services.
///
/// Keeping the transport shape in one struct ensures every failure path emits
/// the same `status/error/message` contract.
#[derive(Debug)]
pub(crate) struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) error: String,
    pub(crate) message: String,
}
