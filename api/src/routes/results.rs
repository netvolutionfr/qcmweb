use axum::extract::{Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::IntoResponse;
use axum::Json;
use time::OffsetDateTime;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::Author;
use crate::domain::grading::Grading;
use crate::domain::results::{self, AttemptFacts, ParticipantFacts, ResultsTable};
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(table))
        .routes(routes!(csv))
}

/// Tableau de résultats d'une évaluation.
///
/// Pseudonymisé par construction : les lignes portent des jetons. La jointure
/// avec les noms s'effectue dans le navigateur de l'enseignant, à partir de sa
/// liste locale (ADR-0001).
#[utoipa::path(
    get,
    path = "/api/assessments/{id}/results",
    responses((status = 200, body = ResultsTable), (status = 404)),
    tag = "resultats",
)]
async fn table(
    _: Author,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ResultsTable>, AppError> {
    Ok(Json(collect(&state, id).await?))
}

/// Même tableau en CSV, prêt pour un tableur.
#[utoipa::path(
    get,
    path = "/api/assessments/{id}/results.csv",
    responses((status = 200, content_type = "text/csv"), (status = 404)),
    tag = "resultats",
)]
async fn csv(
    _: Author,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let table = collect(&state, id).await?;

    let (name,): (String,) = sqlx::query_as("SELECT name FROM assessments WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let filename = format!(
        "resultats-{}.csv",
        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
    );

    Ok((
        [
            (CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        results::to_csv(&table),
    ))
}

// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct AttemptRow {
    participant_id: Uuid,
    started_at: OffsetDateTime,
    submitted_at: Option<OffsetDateTime>,
    score: Option<f64>,
    max_score: Option<f64>,
    grade: Option<f64>,
    breakdown: Option<serde_json::Value>,
}

/// Rassemble les faits, puis délègue l'agrégation au domaine.
///
/// Les participants sont lus depuis le **groupe** de l'évaluation et non depuis
/// les tentatives : c'est ce qui fait apparaître les absents, qui seraient
/// invisibles dans une simple jointure sur les copies.
pub(crate) async fn collect(state: &AppState, id: Uuid) -> Result<ResultsTable, AppError> {
    let participants: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT p.id, p.token
           FROM participants p
           JOIN assessments a ON a.group_id = p.group_id
          WHERE a.id = $1 AND p.active
          ORDER BY p.token",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    if participants.is_empty() {
        let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM assessments WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
        exists.ok_or(AppError::NotFound)?;
    }

    let attempts: Vec<AttemptRow> = sqlx::query_as(
        "SELECT participant_id, started_at, submitted_at, score, max_score, grade, breakdown
           FROM attempts WHERE assessment_id = $1",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let participants: Vec<ParticipantFacts> = participants
        .into_iter()
        .map(|(id, token)| ParticipantFacts { id, token })
        .collect();

    let attempts: Vec<AttemptFacts> = attempts
        .into_iter()
        .map(|row| AttemptFacts {
            participant_id: row.participant_id,
            started_at: row.started_at,
            submitted_at: row.submitted_at,
            score: row.score,
            max_score: row.max_score,
            grade: row.grade,
            breakdown: row
                .breakdown
                .and_then(|value| serde_json::from_value::<Grading>(value).ok()),
        })
        .collect();

    Ok(results::build(&participants, &attempts))
}
