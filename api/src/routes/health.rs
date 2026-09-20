use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct Health {
    status: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/health", get(health))
}

/// Vérifie que l'API répond et que la base est joignable.
#[utoipa::path(
    get,
    path = "/api/health",
    responses((status = 200, body = Health)),
)]
async fn health(State(state): State<AppState>) -> Result<Json<Health>, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(Health { status: "ok" }))
}
