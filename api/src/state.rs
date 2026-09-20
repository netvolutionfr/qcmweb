use std::sync::Arc;

use sqlx::PgPool;

use crate::auth::LoginLimiter;
use crate::config::AuthConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub auth: Arc<AuthConfig>,
    pub limiter: Arc<LoginLimiter>,
}
