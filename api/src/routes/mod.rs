pub mod auth;
pub mod groups;
pub mod health;

use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

/// Métadonnées du document OpenAPI.
///
/// Les chemins et les schémas ne sont pas énumérés ici : ils sont collectés
/// depuis les annotations par `OpenApiRouter`. Une liste tenue à la main
/// finirait par diverger des routes réellement montées, et le client
/// TypeScript hériterait silencieusement de cette dérive.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "QCMWeb",
        description = "API d'évaluation par QCM. Le serveur ne détient aucune donnée nominative : \
                       les participants y sont désignés par un jeton.",
        license(name = "MIT"),
    ),
    tags(
        (name = "authentification", description = "Session enseignant et clés d'agent"),
        (name = "groupes", description = "Groupes et jetons de participation"),
        (name = "systeme", description = "Supervision"),
    ),
)]
pub struct ApiDoc;

pub fn router(state: AppState) -> Router {
    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(auth::router())
        .merge(groups::router())
        .merge(health::router())
        .split_for_parts();

    // Sérialisé une fois au démarrage : le document ne change pas d'une requête
    // à l'autre, et le regénérer à chaque appel serait du travail pur perte.
    let document = serde_json::to_string(&openapi).expect("document OpenAPI sérialisable");

    router
        .route(
            "/api/openapi.json",
            get(move || {
                let document = document.clone();
                async move {
                    ([(axum::http::header::CONTENT_TYPE, "application/json")], document)
                        .into_response()
                }
            }),
        )
        .with_state(state)
}
