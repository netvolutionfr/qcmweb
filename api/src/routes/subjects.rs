use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::audit;
use crate::auth::{Author, Teacher};
use crate::domain::qcm::{Document, ValidationError};
use crate::error::AppError;
use crate::state::AppState;

/// Garde-fou sur la taille d'un document déposé. Un QCM de séance pèse quelques
/// kilo-octets ; au-delà, c'est une erreur d'envoi, pas un sujet.
const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(validate))
        .routes(routes!(list, deposit))
        .routes(routes!(detail))
        .routes(routes!(add_version))
        .routes(routes!(version_document))
        .routes(routes!(approve))
        .routes(routes!(archive))
}

// ---------------------------------------------------------------------------
// Représentations
// ---------------------------------------------------------------------------

#[derive(Serialize, utoipa::ToSchema)]
pub struct Verdict {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    /// Renseigné seulement si le document est valide.
    pub summary: Option<Summary>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Summary {
    pub title: String,
    pub question_count: i32,
    pub total_points: f64,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SubjectVersion {
    pub number: i32,
    /// `DRAFT` ou `VALIDATED`.
    pub status: String,
    pub title: String,
    pub question_count: i32,
    pub total_points: f64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub validated_at: Option<OffsetDateTime>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Subject {
    pub id: Uuid,
    /// Titre de la version la plus récente.
    pub title: String,
    pub latest_version: i32,
    pub latest_status: String,
    pub archived: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SubjectDetail {
    #[serde(flatten)]
    pub subject: Subject,
    pub versions: Vec<SubjectVersion>,
}

/// Sujet déposé : identifiant, version créée, et l'URL de relecture à rendre à
/// l'enseignant (SPEC §9).
#[derive(Serialize, utoipa::ToSchema)]
pub struct Deposited {
    pub subject_id: Uuid,
    pub version: i32,
    pub status: String,
    pub review_url: String,
    pub summary: Summary,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ArchiveTarget {
    pub archived: bool,
}

// ---------------------------------------------------------------------------
// Lecture et validation
// ---------------------------------------------------------------------------

/// Analyse un document `qcm/v1` sans rien écrire.
///
/// Le corps est du texte brut : YAML 1.2 étant un sur-ensemble de JSON, un même
/// point d'entrée accepte les deux formats sans négociation de type.
///
/// Renvoie **toutes** les erreurs d'un coup : un agent qui corrige son fichier a
/// besoin de la liste complète, pas de la première anomalie rencontrée.
#[utoipa::path(
    post,
    path = "/api/subjects/validate",
    request_body(content = String, description = "Document qcm/v1 en YAML ou JSON"),
    responses((status = 200, body = Verdict)),
    tag = "sujets",
)]
async fn validate(_: Author, body: String) -> Result<Json<Verdict>, AppError> {
    Ok(Json(match parse(&body) {
        Ok(document) => Verdict {
            valid: true,
            errors: vec![],
            summary: Some(summarize(&document)),
        },
        Err(errors) => Verdict {
            valid: false,
            errors,
            summary: None,
        },
    }))
}

/// Liste les sujets de la banque.
#[utoipa::path(get, path = "/api/subjects", responses((status = 200, body = [Subject])), tag = "sujets")]
async fn list(_: Author, State(state): State<AppState>) -> Result<Json<Vec<Subject>>, AppError> {
    Ok(Json(list_subjects(&state).await?))
}

pub(crate) async fn list_subjects(state: &AppState) -> Result<Vec<Subject>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, OffsetDateTime, Option<OffsetDateTime>, i32, String, String)>(
        "SELECT s.id, s.created_at, s.archived_at, v.number, v.status, v.title
           FROM subjects s
           JOIN LATERAL (
                SELECT number, status, title FROM subject_versions
                 WHERE subject_id = s.id ORDER BY number DESC LIMIT 1
           ) v ON true
          ORDER BY s.created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(
        rows.into_iter()
            .map(|(id, created_at, archived_at, number, status, title)| Subject {
                id,
                title,
                latest_version: number,
                latest_status: status,
                archived: archived_at.is_some(),
                created_at,
            })
            .collect(),
    )
}

/// Détail d'un sujet et de toutes ses versions.
#[utoipa::path(
    get,
    path = "/api/subjects/{id}",
    responses((status = 200, body = SubjectDetail), (status = 404)),
    tag = "sujets",
)]
async fn detail(
    _: Author,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SubjectDetail>, AppError> {
    Ok(Json(subject_detail(&state, id).await?))
}

pub(crate) async fn subject_detail(state: &AppState, id: Uuid) -> Result<SubjectDetail, AppError> {
    let (created_at, archived_at) =
        sqlx::query_as::<_, (OffsetDateTime, Option<OffsetDateTime>)>(
            "SELECT created_at, archived_at FROM subjects WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let versions = versions_of(state, id).await?;
    let latest = versions.first().ok_or(AppError::NotFound)?;

    Ok(SubjectDetail {
        subject: Subject {
            id,
            title: latest.title.clone(),
            latest_version: latest.number,
            latest_status: latest.status.clone(),
            archived: archived_at.is_some(),
            created_at,
        },
        versions,
    })
}

/// Document complet d'une version, tel qu'il a été déposé.
///
/// Réservé aux principaux authentifiés : il contient les bonnes réponses et les
/// explications, qui ne doivent jamais atteindre le navigateur d'un élève
/// (SPEC §12).
#[utoipa::path(
    get,
    path = "/api/subjects/{id}/versions/{number}",
    responses((status = 200, body = Object), (status = 404)),
    tag = "sujets",
)]
async fn version_document(
    _: Author,
    State(state): State<AppState>,
    Path((id, number)): Path<(Uuid, i32)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (document,) = sqlx::query_as::<_, (serde_json::Value,)>(
        "SELECT document FROM subject_versions WHERE subject_id = $1 AND number = $2",
    )
    .bind(id)
    .bind(number)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(document))
}

// ---------------------------------------------------------------------------
// Écriture
// ---------------------------------------------------------------------------

/// Dépose un nouveau sujet. Le résultat est **toujours un brouillon**.
#[utoipa::path(
    post,
    path = "/api/subjects",
    request_body(content = String, description = "Document qcm/v1 en YAML ou JSON"),
    responses((status = 200, body = Deposited), (status = 422, description = "document invalide")),
    tag = "sujets",
)]
async fn deposit(
    author: Author,
    State(state): State<AppState>,
    body: String,
) -> Result<Json<Deposited>, AppError> {
    Ok(Json(deposit_document(&state, author, &body).await?))
}

/// Dépôt d'un sujet, partagé par la façade REST et la façade MCP.
///
/// Les deux façades appellent la même fonction : dupliquer la logique
/// garantirait qu'elles divergent, et c'est précisément ici que se trouve la
/// règle « le dépôt produit toujours un brouillon ».
pub(crate) async fn deposit_document(
    state: &AppState,
    author: Author,
    body: &str,
) -> Result<Deposited, AppError> {
    let document = parse(body).map_err(AppError::Invalid)?;

    let (subject_id,) = sqlx::query_as::<_, (Uuid,)>("INSERT INTO subjects DEFAULT VALUES RETURNING id")
        .fetch_one(&state.db)
        .await?;

    insert_version(state, subject_id, 1, &document).await?;

    audit::record(
        &state.db,
        "subject.deposit",
        Some(subject_id),
        serde_json::json!({ "by": author.as_str(), "version": 1, "title": document.metadata.title }),
    )
    .await;

    Ok(Deposited {
        subject_id,
        version: 1,
        status: "DRAFT".into(),
        review_url: format!("/sujets/{subject_id}/versions/1"),
        summary: summarize(&document),
    })
}

/// Ajoute une version à un sujet existant. Elle est elle aussi un brouillon.
///
/// C'est le seul chemin de modification : une version déjà déposée est
/// immuable, y compris en base (SPEC §5).
#[utoipa::path(
    post,
    path = "/api/subjects/{id}/versions",
    request_body(content = String, description = "Document qcm/v1 en YAML ou JSON"),
    responses((status = 200, body = Deposited), (status = 404), (status = 422)),
    tag = "sujets",
)]
async fn add_version(
    author: Author,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: String,
) -> Result<Json<Deposited>, AppError> {
    let document = parse(&body).map_err(AppError::Invalid)?;

    let (last,) = sqlx::query_as::<_, (Option<i32>,)>(
        "SELECT max(v.number) FROM subjects s
           LEFT JOIN subject_versions v ON v.subject_id = s.id
          WHERE s.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let number = last.ok_or(AppError::NotFound)? + 1;
    insert_version(&state, id, number, &document).await?;

    audit::record(
        &state.db,
        "subject.new_version",
        Some(id),
        serde_json::json!({ "by": author.as_str(), "version": number }),
    )
    .await;

    Ok(Json(Deposited {
        subject_id: id,
        version: number,
        status: "DRAFT".into(),
        review_url: format!("/sujets/{id}/versions/{number}"),
        summary: summarize(&document),
    }))
}

/// Valide une version : `DRAFT → VALIDATED`.
///
/// **Réservé à l'enseignant.** C'est la porte de relecture humaine, et la
/// protection réelle contre un agent qui aurait lu des instructions hostiles
/// (ADR-0003). L'extracteur [`Teacher`] rend son contournement impossible à
/// écrire, pas seulement déconseillé.
#[utoipa::path(
    post,
    path = "/api/subjects/{id}/versions/{number}/validate",
    responses((status = 200, body = SubjectVersion), (status = 401), (status = 404)),
    tag = "sujets",
)]
async fn approve(
    _: Teacher,
    State(state): State<AppState>,
    Path((id, number)): Path<(Uuid, i32)>,
) -> Result<Json<SubjectVersion>, AppError> {
    let updated = sqlx::query(
        "UPDATE subject_versions SET status = 'VALIDATED', validated_at = now()
          WHERE subject_id = $1 AND number = $2 AND status = 'DRAFT'",
    )
    .bind(id)
    .bind(number)
    .execute(&state.db)
    .await?;

    if updated.rows_affected() == 0 {
        // Soit la version n'existe pas, soit elle est déjà validée. Dans les
        // deux cas il n'y a rien à faire et rien à signaler de plus.
        return Err(AppError::NotFound);
    }

    audit::record(
        &state.db,
        "subject.validate",
        Some(id),
        serde_json::json!({ "version": number }),
    )
    .await;

    let versions = versions_of(&state, id).await?;
    versions
        .into_iter()
        .find(|v| v.number == number)
        .map(Json)
        .ok_or(AppError::NotFound)
}

/// Archive ou désarchive un sujet. Réservé à l'enseignant.
#[utoipa::path(
    post,
    path = "/api/subjects/{id}/archive",
    request_body = ArchiveTarget,
    responses((status = 200, body = Subject), (status = 404)),
    tag = "sujets",
)]
async fn archive(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ArchiveTarget>,
) -> Result<Json<Subject>, AppError> {
    let done = sqlx::query("UPDATE subjects SET archived_at = CASE WHEN $2 THEN now() END WHERE id = $1")
        .bind(id)
        .bind(body.archived)
        .execute(&state.db)
        .await?;

    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    audit::record(
        &state.db,
        "subject.archive",
        Some(id),
        serde_json::json!({ "archived": body.archived }),
    )
    .await;

    Ok(Json(subject_detail(&state, id).await?.subject))
}

// ---------------------------------------------------------------------------
// Utilitaires
// ---------------------------------------------------------------------------

/// Lit puis valide un document. Les erreurs de syntaxe et les erreurs métier
/// sont rendues sous la même forme, l'appelant n'ayant pas à les distinguer.
pub(crate) fn parse(body: &str) -> Result<Document, Vec<ValidationError>> {
    if body.len() > MAX_DOCUMENT_BYTES {
        return Err(vec![ValidationError {
            path: String::new(),
            message: format!("document trop volumineux ({} octets)", body.len()),
        }]);
    }

    let document = Document::from_yaml(body).map_err(|e| {
        vec![ValidationError {
            path: String::new(),
            message: format!("document illisible : {e}"),
        }]
    })?;

    document.validate()?;
    Ok(document)
}

fn summarize(document: &Document) -> Summary {
    Summary {
        title: document.metadata.title.clone(),
        question_count: document.questions.len() as i32,
        total_points: document.total_points(),
    }
}

async fn insert_version(
    state: &AppState,
    subject_id: Uuid,
    number: i32,
    document: &Document,
) -> Result<(), AppError> {
    let json = serde_json::to_value(document).map_err(|_| AppError::Internal("document non sérialisable"))?;

    sqlx::query(
        "INSERT INTO subject_versions
             (subject_id, number, document, title, question_count, total_points)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(subject_id)
    .bind(number)
    .bind(json)
    .bind(&document.metadata.title)
    .bind(document.questions.len() as i32)
    .bind(document.total_points())
    .execute(&state.db)
    .await?;

    Ok(())
}

async fn versions_of(state: &AppState, id: Uuid) -> Result<Vec<SubjectVersion>, AppError> {
    let rows = sqlx::query_as::<_, (i32, String, String, i32, f64, OffsetDateTime, Option<OffsetDateTime>)>(
        "SELECT number, status, title, question_count, total_points, created_at, validated_at
           FROM subject_versions WHERE subject_id = $1 ORDER BY number DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(number, status, title, question_count, total_points, created_at, validated_at)| {
                SubjectVersion {
                    number,
                    status,
                    title,
                    question_count,
                    total_points,
                    created_at,
                    validated_at,
                }
            },
        )
        .collect())
}
