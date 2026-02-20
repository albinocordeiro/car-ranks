use uuid::Uuid;

/// Header used by the MVP auth shim to identify the caller's user id.
pub(crate) const USER_ID_HEADER: &str = "x-user-id";

/// Authenticated user context propagated into endpoint handlers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AuthContext {
    pub(crate) user_id: Uuid,
}

impl AuthContext {
    /// Constructs an auth context from a validated user identifier.
    pub(crate) fn from_user_id(user_id: Uuid) -> Self {
        Self { user_id }
    }
}
