use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::audit;
use crate::auth::{Author, Teacher};
use crate::domain::assessment::{self, Mode, Release, State as Phase};
use crate::domain::code;
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(detail))
        .routes(routes!(open))
        .routes(routes!(close))
}

// ---------------------------------------------------------------------------
// Représentations
// ---------------------------------------------------------------------------

#[derive(Deserialize, utoipa::ToSchema)]
pub struct NewAssessment {
    pub subject_id: Uuid,
    /// Version du sujet. Elle doit être validée.
    pub version: i32,
    pub group_id: Uuid,
    pub name: String,
    pub mode: Mode,

    #[serde(default, with = "time::serde::rfc3339::option")]
    pub opens_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub closes_at: Option<OffsetDateTime>,

    pub duration_minutes: Option<i32>,
    #[serde(default = "one")]
    pub max_attempts: i32,

    #[serde(default = "yes")]
    pub shuffle_questions: bool,
    #[serde(default = "yes")]
    pub shuffle_choices: bool,

    #[serde(default = "after_close")]
    pub score_release: Release,
    #[serde(default = "after_close")]
    pub correction_release: Release,

    #[serde(default = "twenty")]
    pub max_grade: f64,
}

fn one() -> i32 {
    1
}
fn yes() -> bool {
    true
}
fn after_close() -> Release {
    Release::AfterClose
}
fn twenty() -> f64 {
    20.0
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Assessment {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub state: String,
    /// Composable à cet instant : état **et** fenêtre planifiée.
    pub available: bool,
    pub mode: String,

    pub group_id: Uuid,
    pub group_label: String,
    pub subject_id: Uuid,
    pub subject_title: String,
    pub subject_version: i32,

    #[serde(with = "time::serde::rfc3339::option")]
    pub opens_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub closes_at: Option<OffsetDateTime>,

    pub duration_minutes: Option<i32>,
    pub max_attempts: i32,
    pub shuffle_questions: bool,
    pub shuffle_choices: bool,
    pub score_release: String,
    pub correction_release: String,
    pub max_grade: f64,
    pub total_points: f64,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Projection d'une évaluation, jointe à son groupe et à sa version de sujet.
///
/// Une structure dérivée plutôt qu'un tuple : `sqlx` ne connaît `FromRow` que
/// pour des tuples courts, et vingt et une colonnes anonymes seraient de toute
/// façon illisibles.
#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    name: String,
    code: String,
    state: String,
    mode: String,
    group_id: Uuid,
    group_label: String,
    subject_id: Uuid,
    subject_title: String,
    subject_version: i32,
    opens_at: Option<OffsetDateTime>,
    closes_at: Option<OffsetDateTime>,
    duration_minutes: Option<i32>,
    max_attempts: i32,
    shuffle_questions: bool,
    shuffle_choices: bool,
    score_release: String,
    correction_release: String,
    max_grade: f64,
    total_points: f64,
    created_at: OffsetDateTime,
}

/// Les requêtes sont des constantes : `sqlx` refuse les chaînes construites à
/// l'exécution, ce qui ferme la porte à l'injection par construction.
const SELECT_ALL: &str = "
    SELECT a.id, a.name, a.code, a.state, a.mode,
           g.id AS group_id, g.label AS group_label,
           s.id AS subject_id, v.title AS subject_title, v.number AS subject_version,
           a.opens_at, a.closes_at, a.duration_minutes, a.max_attempts,
           a.shuffle_questions, a.shuffle_choices,
           a.score_release, a.correction_release,
           a.max_grade, v.total_points, a.created_at
      FROM assessments a
      JOIN groups g ON g.id = a.group_id
      JOIN subject_versions v ON v.id = a.subject_version_id
      JOIN subjects s ON s.id = v.subject_id
     ORDER BY a.created_at DESC
";

const SELECT_ONE: &str = "
    SELECT a.id, a.name, a.code, a.state, a.mode,
           g.id AS group_id, g.label AS group_label,
           s.id AS subject_id, v.title AS subject_title, v.number AS subject_version,
           a.opens_at, a.closes_at, a.duration_minutes, a.max_attempts,
           a.shuffle_questions, a.shuffle_choices,
           a.score_release, a.correction_release,
           a.max_grade, v.total_points, a.created_at
      FROM assessments a
      JOIN groups g ON g.id = a.group_id
      JOIN subject_versions v ON v.id = a.subject_version_id
      JOIN subjects s ON s.id = v.subject_id
     WHERE a.id = $1
";

fn present(row: Row, now: OffsetDateTime) -> Assessment {
    let phase = Phase::parse(&row.state).unwrap_or(Phase::Draft);
    Assessment {
        available: assessment::is_available(phase, row.opens_at, row.closes_at, now),
        id: row.id,
        name: row.name,
        code: row.code,
        state: row.state,
        mode: row.mode,
        group_id: row.group_id,
        group_label: row.group_label,
        subject_id: row.subject_id,
        subject_title: row.subject_title,
        subject_version: row.subject_version,
        opens_at: row.opens_at,
        closes_at: row.closes_at,
        duration_minutes: row.duration_minutes,
        max_attempts: row.max_attempts,
        shuffle_questions: row.shuffle_questions,
        shuffle_choices: row.shuffle_choices,
        score_release: row.score_release,
        correction_release: row.correction_release,
        max_grade: row.max_grade,
        total_points: row.total_points,
        created_at: row.created_at,
    }
}

// ---------------------------------------------------------------------------
// Lecture
// ---------------------------------------------------------------------------

/// Liste les évaluations.
#[utoipa::path(get, path = "/api/assessments", responses((status = 200, body = [Assessment])), tag = "evaluations")]
async fn list(_: Author, State(state): State<AppState>) -> Result<Json<Vec<Assessment>>, AppError> {
    let rows = sqlx::query_as::<_, Row>(SELECT_ALL)
        .fetch_all(&state.db)
        .await?;

    let now = OffsetDateTime::now_utc();
    Ok(Json(rows.into_iter().map(|r| present(r, now)).collect()))
}

/// Détail d'une évaluation.
#[utoipa::path(
    get,
    path = "/api/assessments/{id}",
    responses((status = 200, body = Assessment), (status = 404)),
    tag = "evaluations",
)]
async fn detail(
    _: Author,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Assessment>, AppError> {
    Ok(Json(fetch(&state, id).await?))
}

// ---------------------------------------------------------------------------
// Écriture
// ---------------------------------------------------------------------------

/// Crée une évaluation et génère son code.
///
/// Accessible à un agent : le pouvoir reste borné puisque la version doit être
/// **validée** — garantie posée en base — et que l'évaluation naît fermée.
/// Seul l'enseignant peut ensuite l'ouvrir (ADR-0003).
#[utoipa::path(
    post,
    path = "/api/assessments",
    request_body = NewAssessment,
    responses(
        (status = 200, body = Assessment),
        (status = 404, description = "sujet, version ou groupe inconnu"),
        (status = 409, description = "la version n'est pas validée"),
    ),
    tag = "evaluations",
)]
async fn create(
    author: Author,
    State(state): State<AppState>,
    Json(body): Json<NewAssessment>,
) -> Result<Json<Assessment>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("nom d'évaluation vide".into()));
    }
    if body.max_attempts < 1 {
        return Err(AppError::BadRequest("au moins une tentative".into()));
    }
    if body.duration_minutes.is_some_and(|d| d <= 0) {
        return Err(AppError::BadRequest("durée nulle ou négative".into()));
    }
    if let (Some(start), Some(end)) = (body.opens_at, body.closes_at) {
        if end <= start {
            return Err(AppError::BadRequest(
                "la fermeture doit suivre l'ouverture".into(),
            ));
        }
    }

    let (version_id,) = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM subject_versions WHERE subject_id = $1 AND number = $2",
    )
    .bind(body.subject_id)
    .bind(body.version)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let id = insert(&state, &body, name, version_id).await?;

    audit::record(
        &state.db,
        "assessment.create",
        Some(id),
        serde_json::json!({
            "by": author.as_str(),
            "name": name,
            "subject": body.subject_id,
            "version": body.version,
            "group": body.group_id,
        }),
    )
    .await;

    Ok(Json(fetch(&state, id).await?))
}

/// Ouvre l'évaluation aux élèves. **Réservé à l'enseignant.**
#[utoipa::path(
    post,
    path = "/api/assessments/{id}/open",
    responses((status = 200, body = Assessment), (status = 401), (status = 404)),
    tag = "evaluations",
)]
async fn open(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Assessment>, AppError> {
    transition(
        &state,
        id,
        "UPDATE assessments SET state = 'OPEN', opened_at = now() WHERE id = $1",
        "assessment.open",
    )
    .await
}

/// Ferme l'évaluation. **Réservé à l'enseignant.**
#[utoipa::path(
    post,
    path = "/api/assessments/{id}/close",
    responses((status = 200, body = Assessment), (status = 401), (status = 404)),
    tag = "evaluations",
)]
async fn close(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Assessment>, AppError> {
    transition(
        &state,
        id,
        "UPDATE assessments SET state = 'CLOSED', closed_at = now() WHERE id = $1",
        "assessment.close",
    )
    .await
}

// ---------------------------------------------------------------------------
// Utilitaires
// ---------------------------------------------------------------------------

async fn fetch(state: &AppState, id: Uuid) -> Result<Assessment, AppError> {
    let row = sqlx::query_as::<_, Row>(SELECT_ONE)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(present(row, OffsetDateTime::now_utc()))
}

async fn transition(
    state: &AppState,
    id: Uuid,
    query: &'static str,
    action: &str,
) -> Result<Json<Assessment>, AppError> {
    let done = sqlx::query(query).bind(id).execute(&state.db).await?;

    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    audit::record(&state.db, action, Some(id), serde_json::json!({})).await;
    Ok(Json(fetch(state, id).await?))
}

/// Insère l'évaluation en retirant un code libre en cas de collision.
///
/// Comme pour les jetons, une collision silencieuse serait pire qu'un échec :
/// deux évaluations partageant un code enverraient une classe composer le sujet
/// d'une autre.
async fn insert(
    state: &AppState,
    body: &NewAssessment,
    name: &str,
    version_id: Uuid,
) -> Result<Uuid, AppError> {
    for _ in 0..5 {
        let code = code::assessment_code();
        let row = sqlx::query_as::<_, (Uuid,)>(
            "INSERT INTO assessments
                 (subject_version_id, group_id, name, mode, code,
                  opens_at, closes_at, duration_minutes, max_attempts,
                  shuffle_questions, shuffle_choices,
                  score_release, correction_release, max_grade)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (code) DO NOTHING
             RETURNING id",
        )
        .bind(version_id)
        .bind(body.group_id)
        .bind(name)
        .bind(serde_json::to_value(body.mode).unwrap().as_str().unwrap())
        .bind(&code)
        .bind(body.opens_at)
        .bind(body.closes_at)
        .bind(body.duration_minutes)
        .bind(body.max_attempts)
        .bind(body.shuffle_questions)
        .bind(body.shuffle_choices)
        .bind(serde_json::to_value(body.score_release).unwrap().as_str().unwrap())
        .bind(serde_json::to_value(body.correction_release).unwrap().as_str().unwrap())
        .bind(body.max_grade)
        .fetch_optional(&state.db)
        .await
        .map_err(translate)?;

        if let Some((id,)) = row {
            return Ok(id);
        }
        tracing::warn!(%code, "collision de code d'évaluation, nouveau tirage");
    }
    Err(AppError::Internal("aucun code libre en cinq essais"))
}

/// Traduit les refus de la base en réponses utiles.
///
/// Le déclencheur qui impose une version validée est la garantie centrale de
/// l'autonomie laissée aux agents : son message mérite mieux qu'un 500 muet.
fn translate(error: sqlx::Error) -> AppError {
    if let Some(db) = error.as_database_error() {
        let message = db.message();
        if message.contains("version validée") {
            return AppError::Conflict(message.to_string());
        }
        if db.is_foreign_key_violation() {
            return AppError::NotFound;
        }
    }
    AppError::Db(error)
}
