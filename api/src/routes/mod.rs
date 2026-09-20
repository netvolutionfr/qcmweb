pub mod assessments;
pub mod auth;
pub mod exam;
pub mod groups;
pub mod health;
pub mod subjects;

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
        (name = "systeme", description = "Supervision"),
    ),
)]
pub struct ApiDoc;

pub fn router(state: AppState) -> Router {
    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(assessments::router())
        .merge(auth::router())
        .merge(exam::router())
        .merge(groups::router())
        .merge(health::router())
        .merge(subjects::router())
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
