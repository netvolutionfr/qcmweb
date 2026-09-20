use axum::extract::{Path, State};
use axum::Json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::audit;
use crate::auth::Teacher;
use crate::domain::{code, school_year};
use crate::error::AppError;
use crate::state::AppState;

/// Borne haute sur une création de jetons. Aucun groupe scolaire n'atteint
/// cette taille : au-delà, c'est une faute de frappe, pas une intention.
const MAX_PARTICIPANTS: i64 = 300;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(list_participants, create_participants))
        .routes(routes!(purge))
        .routes(routes!(reset_secret))
        .routes(routes!(deactivate))
        .routes(routes!(move_to_group))
}

// ---------------------------------------------------------------------------
// Représentations
// ---------------------------------------------------------------------------

#[derive(Deserialize, utoipa::ToSchema)]
pub struct NewGroup {
    /// Étiquette de classe : `1SIO`, `2SIO-SLAM`. Jamais un nom de personne.
    pub label: String,
    /// Année scolaire au format `2026-2027`.
    pub school_year: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Group {
    pub id: Uuid,
    pub label: String,
    pub school_year: String,
    pub participants: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct NewParticipants {
    pub count: i64,
}

/// Jeton fraîchement créé, **secret en clair compris**.
///
/// C'est la seule représentation qui expose un secret, et elle n'est renvoyée
/// qu'une fois. Le serveur n'en conserve ensuite qu'une empreinte Argon2id : il
/// n'existe aucun moyen de réafficher ce secret, seulement de le réinitialiser.
#[derive(Serialize, utoipa::ToSchema)]
pub struct IssuedParticipant {
    pub id: Uuid,
    pub token: String,
    pub secret: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Participant {
    pub id: Uuid,
    pub token: String,
    pub active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct MoveTarget {
    pub group_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Purged {
    pub participants: u64,
}

// ---------------------------------------------------------------------------
// Groupes
// ---------------------------------------------------------------------------

/// Liste les groupes et leur effectif.
#[utoipa::path(get, path = "/api/groups", responses((status = 200, body = [Group])))]
async fn list(_: Teacher, State(state): State<AppState>) -> Result<Json<Vec<Group>>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, OffsetDateTime, i64)>(
        "SELECT g.id, g.label, g.school_year, g.created_at, count(p.id)
           FROM groups g
           LEFT JOIN participants p ON p.group_id = g.id
          GROUP BY g.id
          ORDER BY g.school_year DESC, g.label",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, label, school_year, created_at, participants)| Group {
                id,
                label,
                school_year,
                participants,
                created_at,
            })
            .collect(),
    ))
}

/// Crée un groupe.
#[utoipa::path(
    post,
    path = "/api/groups",
    request_body = NewGroup,
    responses((status = 200, body = Group), (status = 409, description = "groupe déjà existant")),
)]
async fn create(
    _: Teacher,
    State(state): State<AppState>,
    Json(body): Json<NewGroup>,
) -> Result<Json<Group>, AppError> {
    let label = body.label.trim();
    if label.is_empty() {
        return Err(AppError::BadRequest("étiquette de groupe vide".into()));
    }
    school_year::parse(&body.school_year).map_err(AppError::BadRequest)?;

    let row = sqlx::query_as::<_, (Uuid, OffsetDateTime)>(
        "INSERT INTO groups (label, school_year) VALUES ($1, $2)
         ON CONFLICT (label, school_year) DO NOTHING
         RETURNING id, created_at",
    )
    .bind(label)
    .bind(&body.school_year)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        AppError::Conflict(format!(
            "le groupe `{label}` existe déjà pour {}",
            body.school_year
        ))
    })?;

    Ok(Json(Group {
        id: row.0,
        label: label.to_string(),
        school_year: body.school_year,
        participants: 0,
        created_at: row.1,
    }))
}

/// Détruit un groupe, ses jetons et les tentatives associées.
///
/// C'est le chemin de purge de fin d'année exigé par ADR-0002 : une opération
/// prévue et tracée, et non un `DELETE` manuel en base.
#[utoipa::path(post, path = "/api/groups/{id}/purge", responses((status = 200, body = Purged)))]
async fn purge(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Purged>, AppError> {
    let group = sqlx::query_as::<_, (String, String)>(
        "SELECT label, school_year FROM groups WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let participants = sqlx::query("DELETE FROM participants WHERE group_id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();

    sqlx::query("DELETE FROM groups WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    audit::record(
        &state.db,
        "group.purge",
        Some(id),
        serde_json::json!({
            "label": group.0,
            "school_year": group.1,
            "participants": participants,
        }),
    )
    .await;

    Ok(Json(Purged { participants }))
}

// ---------------------------------------------------------------------------
// Participants
// ---------------------------------------------------------------------------

/// Liste les jetons d'un groupe. Ne contient aucun nom, par construction.
#[utoipa::path(
    get,
    path = "/api/groups/{id}/participants",
    responses((status = 200, body = [Participant])),
)]
async fn list_participants(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Participant>>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, String, bool, OffsetDateTime)>(
        "SELECT id, token, active, expires_at FROM participants
          WHERE group_id = $1 ORDER BY created_at, token",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, token, active, expires_at)| Participant {
                id,
                token,
                active,
                expires_at,
            })
            .collect(),
    ))
}

/// Crée `count` jetons dans un groupe et renvoie les secrets en clair.
///
/// Ces secrets ne seront plus jamais affichés : le navigateur de l'enseignant
/// les joint immédiatement à sa liste nominative locale pour produire les
/// billets (ADR-0001).
#[utoipa::path(
    post,
    path = "/api/groups/{id}/participants",
    request_body = NewParticipants,
    responses((status = 200, body = [IssuedParticipant])),
)]
async fn create_participants(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<NewParticipants>,
) -> Result<Json<Vec<IssuedParticipant>>, AppError> {
    if !(1..=MAX_PARTICIPANTS).contains(&body.count) {
        return Err(AppError::BadRequest(format!(
            "effectif attendu entre 1 et {MAX_PARTICIPANTS}, reçu {}",
            body.count
        )));
    }

    let (year,) = sqlx::query_as::<_, (String,)>("SELECT school_year FROM groups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let expires_at = school_year::expiry(&year).map_err(AppError::BadRequest)?;

    let secrets: Vec<String> = (0..body.count).map(|_| code::participant_secret()).collect();

    // On hache la forme canonique, pas la forme affichée : l'élève saisira son
    // secret avec ou sans tiret, en majuscules ou non, et les deux doivent
    // aboutir au même hachage (ADR-0007).
    let hashes = hash_all(secrets.iter().map(|s| code::normalize(s)).collect()).await?;

    let mut issued = Vec::with_capacity(secrets.len());
    for (secret, hash) in secrets.into_iter().zip(hashes) {
        let (pid, token) = insert_participant(&state.db, id, &hash, expires_at).await?;
        issued.push(IssuedParticipant {
            id: pid,
            token,
            secret,
        });
    }

    audit::record(
        &state.db,
        "participants.issue",
        Some(id),
        serde_json::json!({ "count": issued.len() }),
    )
    .await;

    Ok(Json(issued))
}

/// Réinitialise le secret d'un jeton et renvoie le nouveau, une seule fois.
#[utoipa::path(
    post,
    path = "/api/participants/{id}/reset-secret",
    responses((status = 200, body = IssuedParticipant)),
)]
async fn reset_secret(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IssuedParticipant>, AppError> {
    let secret = code::participant_secret();
    let hash = hash_all(vec![code::normalize(&secret)]).await?.remove(0);

    let (token,) = sqlx::query_as::<_, (String,)>(
        "UPDATE participants SET secret_hash = $2 WHERE id = $1 RETURNING token",
    )
    .bind(id)
    .bind(&hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    audit::record(&state.db, "participant.reset_secret", Some(id), serde_json::json!({})).await;

    Ok(Json(IssuedParticipant { id, token, secret }))
}

/// Désactive un jeton : il ne peut plus servir à composer.
#[utoipa::path(
    post,
    path = "/api/participants/{id}/deactivate",
    responses((status = 200, body = Participant)),
)]
async fn deactivate(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Participant>, AppError> {
    let row = sqlx::query_as::<_, (String, bool, OffsetDateTime)>(
        "UPDATE participants SET active = false WHERE id = $1
         RETURNING token, active, expires_at",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    audit::record(&state.db, "participant.deactivate", Some(id), serde_json::json!({})).await;

    Ok(Json(Participant {
        id,
        token: row.0,
        active: row.1,
        expires_at: row.2,
    }))
}

/// Transfère un jeton vers un autre groupe.
///
/// L'échéance est recalculée sur l'année scolaire du groupe d'arrivée : un
/// transfert ne doit pas prolonger silencieusement la durée de conservation.
#[utoipa::path(
    post,
    path = "/api/participants/{id}/move",
    request_body = MoveTarget,
    responses((status = 200, body = Participant)),
)]
async fn move_to_group(
    _: Teacher,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveTarget>,
) -> Result<Json<Participant>, AppError> {
    let (year,) = sqlx::query_as::<_, (String,)>("SELECT school_year FROM groups WHERE id = $1")
        .bind(body.group_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let expires_at = school_year::expiry(&year).map_err(AppError::BadRequest)?;

    let row = sqlx::query_as::<_, (String, bool)>(
        "UPDATE participants SET group_id = $2, expires_at = $3 WHERE id = $1
         RETURNING token, active",
    )
    .bind(id)
    .bind(body.group_id)
    .bind(expires_at)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    audit::record(
        &state.db,
        "participant.move",
        Some(id),
        serde_json::json!({ "to_group": body.group_id }),
    )
    .await;

    Ok(Json(Participant {
        id,
        token: row.0,
        active: row.1,
        expires_at,
    }))
}

// ---------------------------------------------------------------------------
// Utilitaires
// ---------------------------------------------------------------------------

/// Hache une série de secrets hors du runtime async.
///
/// Argon2id coûte environ 225 ms par secret sur le poste de développement :
/// pour une classe de trente élèves, le faire sur un worker Tokio bloquerait
/// l'exécuteur six secondes et gèlerait toutes les autres requêtes.
///
/// ponytail: hachage séquentiel, mesuré à 6,3 s pour 28 jetons. L'API reste
/// réactive pendant ce temps, mais l'appelant attend. Si l'attente devient
/// gênante, répartir sur `std::thread::available_parallelism()` — l'opération
/// est parfaitement parallèle. Non fait tant qu'elle reste annuelle.
async fn hash_all(secrets: Vec<String>) -> Result<Vec<String>, AppError> {
    tokio::task::spawn_blocking(move || {
        secrets
            .iter()
            .map(|s| crate::admin::hash_password(s))
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|_| AppError::Internal("tâche de hachage interrompue"))?
    .map_err(|_| AppError::Internal("hachage du secret impossible"))
}

/// Insère un participant, en retirant un nouveau jeton en cas de collision.
///
/// Une collision sur 39 bits est très improbable, mais `ON CONFLICT DO NOTHING`
/// la rendrait silencieuse : le groupe recevrait un jeton de moins que demandé,
/// et un élève se retrouverait sans billet le jour de l'évaluation.
async fn insert_participant(
    db: &sqlx::PgPool,
    group_id: Uuid,
    secret_hash: &str,
    expires_at: OffsetDateTime,
) -> Result<(Uuid, String), AppError> {
    for _ in 0..5 {
        let token = code::participant_token();
        let row = sqlx::query_as::<_, (Uuid,)>(
            "INSERT INTO participants (group_id, token, secret_hash, expires_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (token) DO NOTHING
             RETURNING id",
        )
        .bind(group_id)
        .bind(&token)
        .bind(secret_hash)
        .bind(expires_at)
        .fetch_optional(db)
        .await?;

        if let Some((id,)) = row {
            return Ok((id, token));
        }
        tracing::warn!(%token, "collision de jeton, nouveau tirage");
    }
    Err(AppError::Internal(
        "impossible de tirer un jeton libre en cinq essais",
    ))
}
