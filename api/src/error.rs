use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::domain::qcm::ValidationError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("ressource introuvable")]
    NotFound,

    #[error("document invalide")]
    Invalid(Vec<ValidationError>),

    #[error("authentification requise")]
    Unauthorized,

    #[error("accès refusé")]
    Forbidden,

    #[error("trop de tentatives")]
    TooManyRequests,

    #[error("erreur interne")]
    Internal(&'static str),

    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, json!({ "error": self.to_string() })),
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({ "error": self.to_string() }))
            }
            AppError::Forbidden => (StatusCode::FORBIDDEN, json!({ "error": self.to_string() })),
            AppError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                json!({ "error": self.to_string() }),
            ),
            // Le motif est journalisé, jamais renvoyé : il décrit la
            // configuration du serveur, pas la requête du client.
            AppError::Internal(why) => {
                tracing::error!(reason = why, "erreur interne");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "erreur interne" }),
                )
            }
            AppError::Invalid(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "error": self.to_string(), "details": errors }),
            ),
            // Le détail d'une erreur base ne sort jamais vers le client.
            AppError::Db(e) => {
                tracing::error!(error = %e, "erreur base de données");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "erreur interne" }),
                )
            }
        };
        (status, Json(body)).into_response()
    }
}
