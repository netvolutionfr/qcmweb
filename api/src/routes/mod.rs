pub mod auth;
pub mod groups;
pub mod health;

use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(auth::router())
        .merge(groups::router())
        .merge(health::router())
        .with_state(state)
}
