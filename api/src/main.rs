mod admin;
mod audit;
mod auth;
mod config;
mod domain;
mod error;
mod routes;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::auth::LoginLimiter;
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Charge le .env de la racine du dépôt : dotenvy remonte l'arborescence,
    // on peut donc lancer depuis api/ comme depuis la racine. Absent en
    // production, où les variables viennent de l'environnement du conteneur.
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return admin::run(&args).await;
    }

    serve().await
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    match auth::purge_expired_sessions(&db).await {
        Ok(n) if n > 0 => tracing::info!(count = n, "sessions expirées purgées"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "purge des sessions impossible"),
    }

    let state = AppState {
        db,
        auth: Arc::new(config.auth.clone()),
        limiter: Arc::new(LoginLimiter::new()),
    };

    let app = routes::router(state).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "API démarrée");

    // `into_make_service_with_connect_info` : la limitation des tentatives de
    // connexion a besoin de l'adresse du pair.
    // ponytail: derrière un reverse proxy, toutes les requêtes porteront l'IP
    // du proxy. Lire X-Forwarded-For le jour où il y en a un, et uniquement
    // s'il est de confiance.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;

    Ok(())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("arrêt demandé");
}
