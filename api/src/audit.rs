//! Journal des opérations sensibles.

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Consigne une opération.
///
/// N'échoue jamais bruyamment : un journal indisponible ne doit pas empêcher
/// une purge demandée par l'enseignant d'aboutir. L'échec est tracé et
/// l'opération se poursuit.
pub async fn record(db: &PgPool, action: &str, subject_id: Option<Uuid>, details: Value) {
    let done = sqlx::query("INSERT INTO audit_events (action, subject_id, details) VALUES ($1, $2, $3)")
        .bind(action)
        .bind(subject_id)
        .bind(&details)
        .execute(db)
        .await;

    match done {
        Ok(_) => tracing::info!(action, ?subject_id, %details, "audit"),
        Err(e) => tracing::error!(action, error = %e, "journalisation impossible"),
    }
}
