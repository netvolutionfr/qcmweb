//! Parcours élève : connexion, code, composition, remise, résultat.
//!
//! Le serveur est l'unique source de vérité (SPEC §12). Le navigateur n'envoie
//! jamais ni horodatage, ni score, ni identité : il envoie des réponses.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::header::SET_COOKIE;
use axum::response::{AppendHeaders, IntoResponse};
use axum::Json;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::auth::{self, Participant, SESSION_COOKIE, SESSION_TTL};
use crate::domain::assessment::{self, Release, State as Phase};
use crate::domain::exam::{self, Exam};
use crate::domain::grading::{self, Grading};
use crate::domain::{code, qcm::Document};
use crate::error::AppError;
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(whoami))
        .routes(routes!(join))
        .routes(routes!(start))
        .routes(routes!(answer))
        .routes(routes!(submit))
        .routes(routes!(result))
}

// ---------------------------------------------------------------------------
// Représentations
// ---------------------------------------------------------------------------

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TokenCredentials {
    pub token: String,
    pub secret: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Briefing {
    pub assessment_id: Uuid,
    pub name: String,
    pub subject_title: String,
    pub description: Option<String>,
    pub total_points: f64,
    pub question_count: i32,
    pub duration_minutes: Option<i32>,
    /// Tentatives restantes, aménagements compris.
    pub attempts_left: i32,
    /// Tentative déjà commencée et non remise, à reprendre.
    pub resumable_attempt: Option<Uuid>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Sitting {
    pub attempt_id: Uuid,
    pub exam: Exam,
    /// Réponses déjà enregistrées, pour reprendre où l'on s'était arrêté.
    pub answers: HashMap<String, Vec<String>>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deadline: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub server_time: OffsetDateTime,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GivenAnswer {
    pub choices: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Saved {
    pub saved: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub server_time: OffsetDateTime,
}

/// Retour fait à l'élève, taillé selon les règles de l'évaluation.
///
/// Les champs absents ne sont pas masqués à l'affichage : ils ne sont pas
/// calculés dans la réponse (SPEC §10).
#[derive(Serialize, utoipa::ToSchema)]
pub struct AttemptResult {
    pub submitted: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<OffsetDateTime>,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub percentage: Option<f64>,
    pub grade: Option<f64>,
    pub max_grade: Option<f64>,
    /// Détail par question, si la correction est divulgable.
    pub correction: Option<Grading>,
}

// ---------------------------------------------------------------------------
// Connexion
// ---------------------------------------------------------------------------

/// Authentifie un participant par jeton et secret.
#[utoipa::path(
    post,
    path = "/api/auth/token",
    request_body = TokenCredentials,
    responses((status = 200), (status = 401), (status = 429)),
    tag = "eleve",
)]
async fn login(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(creds): Json<TokenCredentials>,
) -> Result<impl IntoResponse, AppError> {
    let ip = auth::client_ip(peer.ip(), &headers, &state.trusted_proxies);
    if !state.limiter.allow(ip) {
        return Err(AppError::TooManyRequests);
    }

    // Saisie normalisée : minuscules, tiret oublié ou mal placé. Refuser ces
    // saisies serait une mauvaise façon d'imposer un format que nous avons
    // choisi (ADR-0007).
    let token = code::normalize(&creds.token);
    let secret = code::normalize(&creds.secret);

    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, secret_hash FROM participants
          WHERE replace(token, '-', '') = $1 AND active AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?;

    let Some((id, hash)) = row else {
        tracing::warn!(%ip, "connexion élève refusée : jeton inconnu");
        return Err(AppError::Unauthorized);
    };

    let secret_matches = {
        let hash = hash.clone();
        tokio::task::spawn_blocking(move || auth::verify_password(&secret, &hash))
            .await
            .map_err(|_| AppError::Internal("vérification interrompue"))??
    };

    if !secret_matches {
        tracing::warn!(%ip, "connexion élève refusée : secret invalide");
        return Err(AppError::Unauthorized);
    }

    state.limiter.reset(ip);
    let session = auth::open_participant_session(&state.db, id).await?;

    Ok((
        AppendHeaders([(
            SET_COOKIE,
            format!(
                "{SESSION_COOKIE}={session}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
                SESSION_TTL.whole_seconds()
            ),
        )]),
        Json(serde_json::json!({ "authenticated": true })),
    ))
}

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

/// Vérifie qu'une session de participant est valide.
///
/// Pendant du `/api/auth/me` de l'enseignant. Sans lui, le front devrait
/// sonder une autre route et interpréter son refus, ce qui mêlerait deux
/// significations dans un même code de statut.
#[utoipa::path(
    get,
    path = "/api/auth/participant",
    responses((status = 200), (status = 401)),
    tag = "eleve",
)]
async fn whoami(participant: Participant) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "authenticated": true,
        "group_id": participant.group_id,
    }))
}

/// Présente les consignes d'une évaluation à partir de son code.
#[utoipa::path(
    post,
    path = "/api/join/{code}",
    responses((status = 200, body = Briefing), (status = 403), (status = 404)),
    tag = "eleve",
)]
async fn join(
    participant: Participant,
    State(state): State<AppState>,
    Path(entered): Path<String>,
) -> Result<Json<Briefing>, AppError> {
    let assessment = open_assessment(&state, &entered).await?;

    // Appartenance au groupe : lue sur la session, jamais fournie par le
    // client (SPEC §17).
    if assessment.group_id != participant.group_id {
        tracing::warn!(participant = %participant.id, "code saisi hors de son groupe");
        return Err(AppError::Forbidden);
    }

    let (used, extra_attempts) = attempt_budget(&state, &assessment, participant.id).await?;

    let resumable: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM attempts
          WHERE assessment_id = $1 AND participant_id = $2 AND submitted_at IS NULL
          ORDER BY started_at DESC LIMIT 1",
    )
    .bind(assessment.id)
    .bind(participant.id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(Briefing {
        assessment_id: assessment.id,
        name: assessment.name.clone(),
        subject_title: assessment.document.metadata.title.clone(),
        description: assessment.document.metadata.description.clone(),
        total_points: assessment.document.total_points(),
        question_count: assessment.document.questions.len() as i32,
        duration_minutes: assessment.duration_minutes,
        attempts_left: (assessment.max_attempts + extra_attempts - used).max(0),
        resumable_attempt: resumable.map(|(id,)| id),
    }))
}

/// Démarre une tentative, ou reprend celle qui est en cours.
#[utoipa::path(
    post,
    path = "/api/assessments/{id}/attempts",
    responses((status = 200, body = Sitting), (status = 403), (status = 409)),
    tag = "eleve",
)]
async fn start(
    participant: Participant,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Sitting>, AppError> {
    let assessment = load(&state, id).await?;

    if assessment.group_id != participant.group_id {
        return Err(AppError::Forbidden);
    }
    if !assessment.available(OffsetDateTime::now_utc()) {
        return Err(AppError::Conflict("évaluation fermée".into()));
    }

    let existing: Option<(Uuid, i64, Option<OffsetDateTime>)> = sqlx::query_as(
        "SELECT id, seed, deadline FROM attempts
          WHERE assessment_id = $1 AND participant_id = $2 AND submitted_at IS NULL
          ORDER BY started_at DESC LIMIT 1",
    )
    .bind(id)
    .bind(participant.id)
    .fetch_optional(&state.db)
    .await?;

    let (attempt_id, seed, deadline) = match existing {
        // Reprise : ni nouvelle graine, ni nouvelle échéance. Redémarrer le
        // chronomètre à chaque rechargement offrirait un temps illimité.
        Some(found) => found,
        None => {
            let (used, extra_attempts) = attempt_budget(&state, &assessment, participant.id).await?;
            if used >= assessment.max_attempts + extra_attempts {
                return Err(AppError::Conflict("nombre de tentatives épuisé".into()));
            }

            let extra_minutes = assessment_extra_minutes(&state, id, participant.id).await?;
            let seed = rand::random::<i64>();
            let deadline = assessment.duration_minutes.map(|minutes| {
                OffsetDateTime::now_utc() + time::Duration::minutes((minutes + extra_minutes) as i64)
            });

            // On relit l'échéance telle qu'elle a été stockée : PostgreSQL
            // tronque à la microseconde, et renvoyer la valeur calculée ferait
            // différer la première réponse de toutes les suivantes.
            let (new_id, stored): (Uuid, Option<OffsetDateTime>) = sqlx::query_as(
                "INSERT INTO attempts (assessment_id, participant_id, seed, deadline)
                 VALUES ($1, $2, $3, $4) RETURNING id, deadline",
            )
            .bind(id)
            .bind(participant.id)
            .bind(seed)
            .bind(deadline)
            .fetch_one(&state.db)
            .await?;

            (new_id, seed, stored)
        }
    };

    Ok(Json(Sitting {
        attempt_id,
        exam: exam::build(
            &assessment.document,
            seed,
            assessment.shuffle_questions,
            assessment.shuffle_choices,
        ),
        answers: load_answers(&state, attempt_id).await?,
        deadline,
        server_time: OffsetDateTime::now_utc(),
    }))
}

/// Enregistre la réponse à une question. Appelé au fil de l'eau (SPEC §12).
#[utoipa::path(
    put,
    path = "/api/attempts/{id}/answers/{question}",
    request_body = GivenAnswer,
    responses((status = 200, body = Saved), (status = 403), (status = 409)),
    tag = "eleve",
)]
async fn answer(
    participant: Participant,
    State(state): State<AppState>,
    Path((id, question)): Path<(Uuid, String)>,
    Json(body): Json<GivenAnswer>,
) -> Result<Json<Saved>, AppError> {
    let attempt = own_attempt(&state, id, &participant).await?;
    let assessment = load(&state, attempt.assessment_id).await?;
    let now = OffsetDateTime::now_utc();

    if attempt.submitted_at.is_some() {
        return Err(AppError::Conflict("copie déjà remise".into()));
    }
    if attempt.deadline.is_some_and(|end| now >= end) {
        return Err(AppError::Conflict("temps écoulé".into()));
    }
    if !assessment.available(now) {
        return Err(AppError::Conflict("évaluation fermée".into()));
    }
    if !assessment.document.questions.iter().any(|q| q.id() == question) {
        return Err(AppError::NotFound);
    }

    sqlx::query(
        "INSERT INTO attempt_answers (attempt_id, question_id, choices)
         VALUES ($1, $2, $3)
         ON CONFLICT (attempt_id, question_id)
         DO UPDATE SET choices = EXCLUDED.choices, updated_at = now()",
    )
    .bind(id)
    .bind(&question)
    .bind(serde_json::to_value(&body.choices).unwrap_or_default())
    .execute(&state.db)
    .await?;

    Ok(Json(Saved {
        saved: true,
        server_time: now,
    }))
}

/// Remet définitivement la copie et déclenche la correction.
#[utoipa::path(
    post,
    path = "/api/attempts/{id}/submit",
    responses((status = 200, body = AttemptResult), (status = 403), (status = 409)),
    tag = "eleve",
)]
async fn submit(
    participant: Participant,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AttemptResult>, AppError> {
    let attempt = own_attempt(&state, id, &participant).await?;
    if attempt.submitted_at.is_some() {
        return Err(AppError::Conflict("copie déjà remise".into()));
    }

    let assessment = load(&state, attempt.assessment_id).await?;
    let answers = load_answers(&state, id).await?;

    // Correction intégralement serveur, sur le document figé de la version
    // référencée par l'évaluation (SPEC §13).
    let grading = grading::grade(&assessment.document, &answers);
    let grade = assessment::grade(grading.score, grading.max_score, assessment.max_grade);

    sqlx::query(
        "UPDATE attempts
            SET submitted_at = now(), score = $2, max_score = $3, grade = $4, breakdown = $5
          WHERE id = $1",
    )
    .bind(id)
    .bind(grading.score)
    .bind(grading.max_score)
    .bind(grade)
    .bind(serde_json::to_value(&grading).unwrap_or_default())
    .execute(&state.db)
    .await?;

    result(participant, State(state), Path(id)).await
}

/// Résultat d'une tentative, selon les règles de divulgation.
#[utoipa::path(
    get,
    path = "/api/attempts/{id}/result",
    responses((status = 200, body = AttemptResult), (status = 403)),
    tag = "eleve",
)]
async fn result(
    participant: Participant,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AttemptResult>, AppError> {
    let attempt = own_attempt(&state, id, &participant).await?;
    let assessment = load(&state, attempt.assessment_id).await?;
    let available = assessment.available(OffsetDateTime::now_utc());

    if attempt.submitted_at.is_none() {
        return Ok(Json(AttemptResult {
            submitted: false,
            submitted_at: None,
            score: None,
            max_score: None,
            percentage: None,
            grade: None,
            max_grade: None,
            correction: None,
        }));
    }

    let show_score = assessment::releases(assessment.score_release, available);
    let show_correction = assessment::releases(assessment.correction_release, available);

    let breakdown: Option<Grading> = attempt
        .breakdown
        .and_then(|value| serde_json::from_value(value).ok());

    Ok(Json(AttemptResult {
        submitted: true,
        submitted_at: attempt.submitted_at,
        score: show_score.then_some(attempt.score).flatten(),
        max_score: show_score.then_some(attempt.max_score).flatten(),
        percentage: show_score
            .then(|| breakdown.as_ref().map(|b| b.percentage))
            .flatten(),
        grade: show_score.then_some(attempt.grade).flatten(),
        max_grade: show_score.then_some(assessment.max_grade),
        correction: show_correction.then_some(breakdown).flatten(),
    }))
}

// ---------------------------------------------------------------------------
// Chargement
// ---------------------------------------------------------------------------

struct Loaded {
    id: Uuid,
    group_id: Uuid,
    name: String,
    document: Document,
    state: Phase,
    opens_at: Option<OffsetDateTime>,
    closes_at: Option<OffsetDateTime>,
    duration_minutes: Option<i32>,
    max_attempts: i32,
    shuffle_questions: bool,
    shuffle_choices: bool,
    score_release: Release,
    correction_release: Release,
    max_grade: f64,
}

impl Loaded {
    fn available(&self, now: OffsetDateTime) -> bool {
        assessment::is_available(self.state, self.opens_at, self.closes_at, now)
    }
}

#[derive(sqlx::FromRow)]
struct AssessmentRow {
    id: Uuid,
    group_id: Uuid,
    name: String,
    document: serde_json::Value,
    state: String,
    opens_at: Option<OffsetDateTime>,
    closes_at: Option<OffsetDateTime>,
    duration_minutes: Option<i32>,
    max_attempts: i32,
    shuffle_questions: bool,
    shuffle_choices: bool,
    score_release: String,
    correction_release: String,
    max_grade: f64,
}

const ASSESSMENT_BY_ID: &str = "
    SELECT a.id, a.group_id, a.name, v.document, a.state,
           a.opens_at, a.closes_at, a.duration_minutes, a.max_attempts,
           a.shuffle_questions, a.shuffle_choices,
           a.score_release, a.correction_release, a.max_grade
      FROM assessments a
      JOIN subject_versions v ON v.id = a.subject_version_id
     WHERE a.id = $1
";

const ASSESSMENT_BY_CODE: &str = "
    SELECT a.id, a.group_id, a.name, v.document, a.state,
           a.opens_at, a.closes_at, a.duration_minutes, a.max_attempts,
           a.shuffle_questions, a.shuffle_choices,
           a.score_release, a.correction_release, a.max_grade
      FROM assessments a
      JOIN subject_versions v ON v.id = a.subject_version_id
     WHERE a.code = $1
";

fn hydrate(row: AssessmentRow) -> Result<Loaded, AppError> {
    Ok(Loaded {
        id: row.id,
        group_id: row.group_id,
        name: row.name,
        document: serde_json::from_value(row.document)
            .map_err(|_| AppError::Internal("document de sujet illisible"))?,
        state: Phase::parse(&row.state).unwrap_or(Phase::Draft),
        opens_at: row.opens_at,
        closes_at: row.closes_at,
        duration_minutes: row.duration_minutes,
        max_attempts: row.max_attempts,
        shuffle_questions: row.shuffle_questions,
        shuffle_choices: row.shuffle_choices,
        score_release: serde_json::from_value(serde_json::Value::String(row.score_release))
            .unwrap_or(Release::AfterClose),
        correction_release: serde_json::from_value(serde_json::Value::String(
            row.correction_release,
        ))
        .unwrap_or(Release::AfterClose),
        max_grade: row.max_grade,
    })
}

async fn load(state: &AppState, id: Uuid) -> Result<Loaded, AppError> {
    let row = sqlx::query_as::<_, AssessmentRow>(ASSESSMENT_BY_ID)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    hydrate(row)
}

/// Charge une évaluation depuis un code saisi.
///
/// Renvoie 404 pour un code inconnu comme pour une évaluation fermée : un
/// message distinct confirmerait l'existence du code et inviterait à
/// l'énumération.
async fn open_assessment(state: &AppState, entered: &str) -> Result<Loaded, AppError> {
    let row = sqlx::query_as::<_, AssessmentRow>(ASSESSMENT_BY_CODE)
        .bind(code::normalize(entered))
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let loaded = hydrate(row)?;
    if !loaded.available(OffsetDateTime::now_utc()) {
        return Err(AppError::NotFound);
    }
    Ok(loaded)
}

struct OwnedAttempt {
    assessment_id: Uuid,
    deadline: Option<OffsetDateTime>,
    submitted_at: Option<OffsetDateTime>,
    score: Option<f64>,
    max_score: Option<f64>,
    grade: Option<f64>,
    breakdown: Option<serde_json::Value>,
}

/// Charge une tentative **en exigeant qu'elle appartienne à l'appelant**.
///
/// Le filtre sur `participant_id` est dans la requête et non dans un contrôle
/// qui suivrait : c'est ce qui interdit à un élève de lire la copie d'un autre
/// en changeant l'identifiant dans l'URL (SPEC §17).
async fn own_attempt(
    state: &AppState,
    id: Uuid,
    participant: &Participant,
) -> Result<OwnedAttempt, AppError> {
    let row: Option<(
        Uuid,
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<serde_json::Value>,
    )> = sqlx::query_as(
        "SELECT assessment_id, deadline, submitted_at, score, max_score, grade, breakdown
           FROM attempts WHERE id = $1 AND participant_id = $2",
    )
    .bind(id)
    .bind(participant.id)
    .fetch_optional(&state.db)
    .await?;

    let row = row.ok_or(AppError::Forbidden)?;
    Ok(OwnedAttempt {
        assessment_id: row.0,
        deadline: row.1,
        submitted_at: row.2,
        score: row.3,
        max_score: row.4,
        grade: row.5,
        breakdown: row.6,
    })
}

async fn load_answers(
    state: &AppState,
    attempt: Uuid,
) -> Result<HashMap<String, Vec<String>>, AppError> {
    let rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT question_id, choices FROM attempt_answers WHERE attempt_id = $1")
            .bind(attempt)
            .fetch_all(&state.db)
            .await?;

    Ok(rows
        .into_iter()
        .map(|(question, choices)| {
            (
                question,
                serde_json::from_value(choices).unwrap_or_default(),
            )
        })
        .collect())
}

/// Tentatives consommées et tentatives supplémentaires accordées.
async fn attempt_budget(
    state: &AppState,
    assessment: &Loaded,
    participant: Uuid,
) -> Result<(i32, i32), AppError> {
    let (used,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM attempts
          WHERE assessment_id = $1 AND participant_id = $2 AND submitted_at IS NOT NULL",
    )
    .bind(assessment.id)
    .bind(participant)
    .fetch_one(&state.db)
    .await?;

    let extra: Option<(i32,)> = sqlx::query_as(
        "SELECT extra_attempts FROM assessment_overrides
          WHERE assessment_id = $1 AND participant_id = $2",
    )
    .bind(assessment.id)
    .bind(participant)
    .fetch_optional(&state.db)
    .await?;

    Ok((used as i32, extra.map(|(n,)| n).unwrap_or(0)))
}

async fn assessment_extra_minutes(
    state: &AppState,
    assessment: Uuid,
    participant: Uuid,
) -> Result<i32, AppError> {
    let extra: Option<(i32,)> = sqlx::query_as(
        "SELECT extra_minutes FROM assessment_overrides
          WHERE assessment_id = $1 AND participant_id = $2",
    )
    .bind(assessment)
    .bind(participant)
    .fetch_optional(&state.db)
    .await?;

    Ok(extra.map(|(n,)| n).unwrap_or(0))
}
