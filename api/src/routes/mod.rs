pub mod assessments;
pub mod auth;
pub mod exam;
pub mod groups;
pub mod health;
pub mod results;
pub mod subjects;

use axum::extract::{FromRequestParts, Request, State};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::StreamableHttpService;

use crate::auth::Agent;
use crate::error::AppError;
use crate::mcp::QcmTools;
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
    // Le modèle qcm/v1 est déclaré explicitement : aucune route ne le référence
    // dans sa signature, puisque les documents transitent en texte brut. Sans
    // cela il serait absent de l'OpenAPI, alors que c'est par là qu'un agent
    // obtient le schéma exigé par la SPEC §9.
    components(schemas(
        crate::domain::qcm::Document,
        crate::domain::qcm::Question,
        crate::domain::qcm::SingleChoice,
        crate::domain::qcm::MultipleChoice,
        crate::domain::qcm::TrueFalse,
        crate::domain::qcm::Choice,
        crate::domain::qcm::Metadata,
        crate::domain::qcm::Teaching,
        crate::domain::qcm::Scoring,
        crate::domain::qcm::Mode,
    )),
    tags(
        (name = "authentification", description = "Session enseignant et clés d'agent"),
        (name = "groupes", description = "Groupes et jetons de participation"),
        (name = "sujets", description = "Banque de sujets et versions"),
        (name = "evaluations", description = "Évaluations et codes d'accès"),
        (name = "eleve", description = "Parcours élève : code, composition, remise"),
        (name = "resultats", description = "Tableau de résultats et exports"),
        (name = "systeme", description = "Supervision"),
    ),
)]
pub struct ApiDoc;

/// Monte la façade MCP sur `/mcp`.
///
/// Même application, même middleware, mêmes fonctions de service que la façade
/// REST (ADR-0003). Seule la porte d'entrée diffère.
fn mcp_route(state: AppState) -> Router<AppState> {
    let service = StreamableHttpService::new(
        {
            let state = state.clone();
            move || Ok(QcmTools::new(state.clone()))
        },
        LocalSessionManager::default().into(),
        Default::default(),
    );

    Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(state, require_agent))
}

/// Exige une clé d'API de scope `agent`.
///
/// On réutilise l'extracteur `Agent` plutôt que de refaire la vérification :
/// deux implémentations de la même règle finiraient par diverger, et c'est
/// celle-ci qui borne ce qu'un agent peut atteindre.
async fn require_agent(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let (mut parts, body) = request.into_parts();
    Agent::from_request_parts(&mut parts, &state).await?;
    Ok(next.run(Request::from_parts(parts, body)).await)
}

/// Le JSON Schema du format natif, extrait du document OpenAPI.
///
/// Un seul artefact décrit le format : le modèle Rust. L'OpenAPI et la
/// ressource MCP en sont deux projections, ce qui interdit à la documentation
/// de diverger du code.
pub fn qcm_schema() -> String {
    let openapi = ApiDoc::openapi();
    let schemas = serde_json::to_value(&openapi)
        .ok()
        .and_then(|v| v.get("components").and_then(|c| c.get("schemas")).cloned())
        .unwrap_or(serde_json::Value::Null);

    serde_json::to_string_pretty(&serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "qcm/v1",
        "$ref": "#/$defs/Document",
        "$defs": schemas,
    }))
    .unwrap_or_default()
}

pub fn router(state: AppState) -> Router {
    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(assessments::router())
        .merge(auth::router())
        .merge(exam::router())
        .merge(groups::router())
        .merge(health::router())
        .merge(results::router())
        .merge(subjects::router())
        .split_for_parts();

    // Sérialisé une fois au démarrage : le document ne change pas d'une requête
    // à l'autre, et le regénérer à chaque appel serait du travail pur perte.
    let document = serde_json::to_string(&openapi).expect("document OpenAPI sérialisable");

    router
        .merge(mcp_route(state.clone()))
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
