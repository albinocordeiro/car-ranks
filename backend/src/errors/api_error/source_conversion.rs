use super::ApiError;

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self::internal(format!("{value}"))
    }
}
