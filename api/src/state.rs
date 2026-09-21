use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Semaphore;

use ipnet::IpNet;

use crate::auth::Throttles;
use crate::config::AuthConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub auth: Arc<AuthConfig>,
    pub throttles: Arc<Throttles>,
    /// Plafond de vérifications Argon2 simultanées (voir `auth::verify_blocking`).
    pub hashing: Arc<Semaphore>,
    pub trusted_proxies: Arc<Vec<IpNet>>,
}
