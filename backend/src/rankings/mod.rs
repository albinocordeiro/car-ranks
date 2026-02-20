use axum::Json;
use axum::extract::{Query, State};

use crate::auth::AuthContext;
use crate::{ApiError, AppState, RankingsQuery, RankingsResponse};

mod postgres;
mod request;

pub(crate) async fn get_rankings(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<RankingsResponse>, ApiError> {
    postgres::get_rankings_postgres(&state, auth, params).await
}
