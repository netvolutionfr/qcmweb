use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::header::SET_COOKIE;
use axum::http::HeaderMap;
use axum::response::{AppendHeaders, IntoResponse};
use axum::Json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use serde::{Deserialize, Serialize};

use crate::auth::{self, Agent, Teacher, SESSION_COOKIE, SESSION_TTL};
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(logout))
        .routes(routes!(me))
        .routes(routes!(agent))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Identity {
    pub authenticated: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentIdentity {
    pub scope: &'static str,
}

/// Ouvre une session enseignant.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = Credentials,
    responses(
        (status = 200, body = Identity),
        (status = 401, description = "identifiants invalides"),
        (status = 429, description = "trop de tentatives"),
    ),
)]
async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(creds): Json<Credentials>,
) -> Result<impl IntoResponse, AppError> {
    let ip = auth::client_ip(peer.ip(), &headers, &state.trusted_proxies);

    // Le quota est réservé avant la vérification : compter après laisserait
    // passer autant d'essais que de requêtes simultanées.
    if !state.throttles.teacher.admit(&ip) {
        tracing::warn!(%ip, "connexion : quota de tentatives dépassé");
        return Err(AppError::TooManyRequests);
    }

    // Le mot de passe est vérifié même lorsque l'identifiant est faux, pour que
    // la durée de la réponse ne révèle pas lequel des deux est en cause.
    let user_ok = creds.username == state.auth.username;
    let pass_ok =
        auth::verify_blocking(&state, &creds.password, &state.auth.password_hash).await?;

    if !(user_ok && pass_ok) {
        tracing::warn!(%ip, "connexion refusée");
        return Err(AppError::Unauthorized);
    }

    // Réussir prouve que l'on connaît le mot de passe de l'enseignant : le
    // quota de cette adresse peut être libéré. C'est le seul endroit où ce
    // compteur est remis à zéro — jamais depuis une autre connexion.
    state.throttles.teacher.clear(&ip);
    let token = auth::open_session(&state.db, &state.auth.credential).await?;
    tracing::info!(%ip, "session ouverte");

    Ok((
        AppendHeaders([(SET_COOKIE, session_cookie(&token, SESSION_TTL))]),
        Json(Identity {
            authenticated: true,
        }),
    ))
}

/// Ferme la session courante.
#[utoipa::path(post, path = "/api/auth/logout", responses((status = 200)))]
async fn logout(
    State(state): State<AppState>,
    jar: axum_extra::extract::CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        auth::close_session(&state.db, cookie.value()).await?;
    }
    Ok((
        AppendHeaders([(SET_COOKIE, session_cookie("", time::Duration::ZERO))]),
        Json(Identity {
            authenticated: false,
        }),
    ))
}

/// Vérifie que la session est valide.
#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses((status = 200, body = Identity), (status = 401)),
)]
async fn me(_: Teacher) -> Json<Identity> {
    Json(Identity {
        authenticated: true,
    })
}

/// Permet à un agent de vérifier que sa clé est valide et de connaître son
/// scope, sans effet de bord.
#[utoipa::path(
    get,
    path = "/api/auth/agent",
    responses((status = 200, body = AgentIdentity), (status = 401)),
)]
async fn agent(agent: Agent) -> Json<AgentIdentity> {
    Json(AgentIdentity {
        scope: agent.scope.as_str(),
    })
}

/// `HttpOnly` pour que le jeton soit hors de portée de tout JavaScript,
/// `SameSite=Strict` parce qu'aucun parcours légitime n'arrive ici depuis un
/// autre site, `Secure` parce que la SPEC impose HTTPS — les navigateurs
/// acceptent ce drapeau sur `localhost` en développement.
fn session_cookie(token: &str, ttl: time::Duration) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        ttl.whole_seconds()
    )
}
