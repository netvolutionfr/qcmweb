//! Authentification des deux principaux de l'application.
//!
//! - **Enseignant** : mot de passe (Argon2id) vérifié une fois, puis une
//!   session opaque portée par un cookie `HttpOnly`.
//! - **Agent** : clé d'API portée par un en-tête `Authorization: Bearer`,
//!   porteuse d'un scope qui borne ce que l'agent peut faire.
//!
//! Le mot de passe enseignant est *fixé* : il n'y a ni inscription, ni
//! réinitialisation en libre-service. L'instance sert un seul enseignant, comme
//! le prévoit la section 18 de la SPEC. Les passkeys remplaceront ce mécanisme
//! dans une version ultérieure.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::error::AppError;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "qcmweb_session";
pub const SESSION_TTL: time::Duration = time::Duration::days(30);
const KEY_PREFIX: &str = "qcmw_";

// ---------------------------------------------------------------------------
// Secrets
// ---------------------------------------------------------------------------

/// 256 bits d'aléa, en hexadécimal.
///
/// `rand::rng()` est un CSPRNG amorcé par le système d'exploitation : c'est le
/// minimum pour un secret d'authentification. Un `u64` tiré d'un générateur
/// ordinaire serait devinable.
fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex(&bytes)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Empreinte d'un secret à pleine entropie.
///
/// SHA-256 et non Argon2id : le coût d'un KDF ne sert qu'à ralentir la
/// recherche exhaustive d'un secret *devinable*. Sur 256 bits aléatoires, il
/// n'apporterait rien et coûterait 100 ms à chaque requête.
pub fn fingerprint(secret: &str) -> String {
    hex(&Sha256::digest(secret.as_bytes()))
}

/// Vérifie un mot de passe contre une empreinte Argon2id au format PHC.
pub fn verify_password(password: &str, phc: &str) -> Result<bool, AppError> {
    let parsed =
        PasswordHash::new(phc).map_err(|_| AppError::Internal("TEACHER_PASSWORD_HASH illisible"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

/// Crée une session et renvoie le jeton en clair — la seule et unique fois.
pub async fn open_session(db: &PgPool) -> Result<String, AppError> {
    let token = random_secret();
    sqlx::query("INSERT INTO sessions (token_hash, expires_at) VALUES ($1, $2)")
        .bind(fingerprint(&token))
        .bind(time::OffsetDateTime::now_utc() + SESSION_TTL)
        .execute(db)
        .await?;
    Ok(token)
}

pub async fn close_session(db: &PgPool, token: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(fingerprint(token))
        .execute(db)
        .await?;
    Ok(())
}

/// Supprime les sessions expirées. Appelé au démarrage : sans cela la table
/// conserverait des sessions mortes, ce qui contredirait la règle de rétention.
pub async fn purge_expired_sessions(db: &PgPool) -> Result<u64, AppError> {
    let done = sqlx::query("DELETE FROM sessions WHERE expires_at < now()")
        .execute(db)
        .await?;
    Ok(done.rows_affected())
}

// ---------------------------------------------------------------------------
// Clés d'API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Rédaction de sujets et lecture de résultats pseudonymisés.
    ///
    /// N'autorise ni `DRAFT -> VALIDATED`, ni l'ouverture d'une évaluation, ni
    /// quoi que ce soit touchant aux participants, aux jetons ou à la purge.
    /// Voir SPEC.md §9.
    Agent,
}

impl Scope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Agent => "agent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "agent" => Some(Scope::Agent),
            _ => None,
        }
    }
}

/// Crée une clé d'API et renvoie le secret en clair — la seule et unique fois.
pub async fn mint_api_key(db: &PgPool, label: &str, scope: Scope) -> Result<String, AppError> {
    let key = format!("{KEY_PREFIX}{}", random_secret());
    sqlx::query("INSERT INTO api_keys (label, key_hash, scope) VALUES ($1, $2, $3)")
        .bind(label)
        .bind(fingerprint(&key))
        .bind(scope.as_str())
        .execute(db)
        .await?;
    Ok(key)
}

// ---------------------------------------------------------------------------
// Extracteurs
// ---------------------------------------------------------------------------

/// Preuve qu'une requête émane de l'enseignant authentifié.
///
/// Un handler qui prend `Teacher` en argument ne peut pas être monté sans
/// authentification par distraction : l'absence du type est une erreur de
/// compilation, pas un oubli de middleware.
pub struct Teacher;

impl FromRequestParts<AppState> for Teacher {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = session_cookie(parts).ok_or(AppError::Unauthorized)?;

        let valid: Option<(uuid::Uuid,)> =
            sqlx::query_as("SELECT id FROM sessions WHERE token_hash = $1 AND expires_at > now()")
                .bind(fingerprint(&token))
                .fetch_optional(&state.db)
                .await?;

        valid.map(|_| Teacher).ok_or(AppError::Unauthorized)
    }
}

/// Preuve qu'une requête émane d'un agent porteur d'une clé valide.
pub struct Agent {
    pub scope: Scope,
}

impl FromRequestParts<AppState> for Agent {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let key = bearer(parts).ok_or(AppError::Unauthorized)?;

        let row: Option<(uuid::Uuid, String)> = sqlx::query_as(
            "SELECT id, scope FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL",
        )
        .bind(fingerprint(&key))
        .fetch_optional(&state.db)
        .await?;

        let (id, scope) = row.ok_or(AppError::Unauthorized)?;
        let scope = Scope::parse(&scope).ok_or(AppError::Forbidden)?;

        // Trace de dernier usage : permet de repérer une clé oubliée, donc à
        // révoquer. Sans valeur de sécurité en soi, mais sans coût non plus.
        let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await;

        Ok(Agent { scope })
    }
}

fn session_cookie(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|c| c.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_string())
}

fn bearer(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|v| v.trim().to_string())
}

// ---------------------------------------------------------------------------
// Limitation des tentatives
// ---------------------------------------------------------------------------

/// Fenêtre fixe par adresse IP sur l'endpoint de connexion (SPEC §17).
///
/// ponytail: compteur en mémoire, remis à zéro au redémarrage et non partagé
/// entre instances. Suffisant pour une instance unique ; passer à Postgres ou
/// Redis le jour où il y en a plusieurs.
///
/// Le comptage est par IP et non global : un attaquant ne doit pas pouvoir
/// verrouiller l'enseignant hors de son propre outil.
pub struct LoginLimiter {
    attempts: Mutex<HashMap<IpAddr, (u32, Instant)>>,
    max: u32,
    window: Duration,
}

impl LoginLimiter {
    pub fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max: 10,
            window: Duration::from_secs(15 * 60),
        }
    }

    /// Enregistre une tentative. `false` si le quota est déjà épuisé.
    pub fn allow(&self, ip: IpAddr) -> bool {
        let mut map = self.attempts.lock().expect("mutex empoisonné");
        let now = Instant::now();

        // Purge opportuniste : évite que la table enfle sous une attaque
        // distribuée. ponytail: O(n) à chaque tentative, négligeable tant que
        // le trafic de connexion reste celui d'un enseignant.
        map.retain(|_, (_, started)| now.duration_since(*started) < self.window);

        let entry = map.entry(ip).or_insert((0, now));
        if now.duration_since(entry.1) >= self.window {
            *entry = (0, now);
        }
        entry.0 += 1;
        entry.0 <= self.max
    }

    /// Une connexion réussie efface le compteur.
    pub fn reset(&self, ip: IpAddr) {
        self.attempts.lock().expect("mutex empoisonné").remove(&ip);
    }
}

impl Default for LoginLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empreinte_stable_et_discriminante() {
        assert_eq!(fingerprint("abc"), fingerprint("abc"));
        assert_ne!(fingerprint("abc"), fingerprint("abd"));
        assert_eq!(fingerprint("abc").len(), 64);
    }

    #[test]
    fn secrets_non_repetes() {
        let a = random_secret();
        let b = random_secret();
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn verification_du_mot_de_passe() {
        // Empreinte de "correct horse" produite par `cargo run --bin admin hash-password`.
        let phc = crate::admin::hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &phc).unwrap());
        assert!(!verify_password("correct hors", &phc).unwrap());
        assert!(!verify_password("", &phc).unwrap());
    }

    #[test]
    fn hash_illisible_refuse_au_lieu_de_laisser_passer() {
        let r = verify_password("peu importe", "pas-un-phc");
        assert!(r.is_err(), "un hash invalide doit échouer, jamais accepter");
    }

    #[test]
    fn limiteur_bloque_apres_le_quota() {
        let limiter = LoginLimiter::new();
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        for i in 1..=10 {
            assert!(limiter.allow(ip), "tentative {i} devrait passer");
        }
        assert!(!limiter.allow(ip), "la 11e doit être refusée");
    }

    #[test]
    fn limiteur_isole_les_adresses() {
        let limiter = LoginLimiter::new();
        let attaquant: IpAddr = "192.0.2.1".parse().unwrap();
        let enseignant: IpAddr = "192.0.2.2".parse().unwrap();

        for _ in 0..20 {
            limiter.allow(attaquant);
        }
        assert!(
            limiter.allow(enseignant),
            "l'enseignant ne doit pas être verrouillé par un tiers"
        );
    }

    #[test]
    fn connexion_reussie_efface_le_compteur() {
        let limiter = LoginLimiter::new();
        let ip: IpAddr = "192.0.2.3".parse().unwrap();
        for _ in 0..10 {
            limiter.allow(ip);
        }
        limiter.reset(ip);
        assert!(limiter.allow(ip));
    }

    #[test]
    fn scope_aller_retour() {
        assert_eq!(Scope::parse("agent"), Some(Scope::Agent));
        assert_eq!(Scope::parse("full"), None);
        assert_eq!(Scope::parse(Scope::Agent.as_str()), Some(Scope::Agent));
    }
}
