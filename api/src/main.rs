mod admin;
mod audit;
mod auth;
mod config;
mod domain;
mod error;
mod mcp;
mod routes;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::auth::Throttles;
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

    // Rotation du mot de passe enseignant : les sessions ouvertes sous l'ancien
    // sont refusées par la validation, on retire aussi leurs lignes.
    match auth::purge_stale_teacher_sessions(&db, &config.auth.credential).await {
        Ok(n) if n > 0 => tracing::warn!(count = n, "sessions enseignant d'anciens identifiants révoquées"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "purge des sessions obsolètes impossible"),
    }

    if config.trusted_proxies.is_empty() {
        tracing::warn!(
            "TRUSTED_PROXY_CIDRS vide : X-Forwarded-For sera ignoré. \
             Correct en accès direct, mais derrière un reverse proxy la \
             limitation des tentatives comptera toutes les connexions ensemble."
        );
    } else {
        tracing::info!(proxies = ?config.trusted_proxies, "proxies de confiance");
    }

    let state = AppState {
        db,
        auth: Arc::new(config.auth.clone()),
        throttles: Arc::new(Throttles::new()),
        hashing: Arc::new(tokio::sync::Semaphore::new(hashing_permits())),
        trusted_proxies: Arc::new(config.trusted_proxies.clone()),
    };

    let app = routes::router(state).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "API démarrée");

    // `into_make_service_with_connect_info` : la limitation des tentatives de
    // connexion a besoin de l'adresse du pair, que `auth::client_ip` complète
    // avec `X-Forwarded-For` lorsque ce pair est un proxy de confiance.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;

    Ok(())
}

/// Vérifications Argon2 simultanées autorisées : autant que de cœurs, au plus
/// quatre. Chacune coûte environ vingt mégaoctets.
fn hashing_permits() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .clamp(1, 4)
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("arrêt demandé");
}
