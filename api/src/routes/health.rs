use axum::extract::State;
use axum::Json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use serde::Serialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct Health {
    status: &'static str,
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(health))
}

/// Vérifie que l'API répond et que la base est joignable.
#[utoipa::path(
    get,
    path = "/api/health",
    responses((status = 200, body = Health)),
    tag = "systeme",
)]
async fn health(State(state): State<AppState>) -> Result<Json<Health>, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(Health { status: "ok" }))
}
