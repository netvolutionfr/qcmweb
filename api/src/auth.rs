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
use uuid::Uuid;

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

        // `participant_id IS NULL` n'est pas un détail : sans cette clause, la
        // session d'un élève ouvrirait les routes enseignantes.
        let valid: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM sessions
              WHERE token_hash = $1 AND expires_at > now() AND participant_id IS NULL",
        )
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

/// Session ouverte pour un participant.
pub async fn open_participant_session(db: &PgPool, participant: Uuid) -> Result<String, AppError> {
    let token = random_secret();
    sqlx::query("INSERT INTO sessions (token_hash, expires_at, participant_id) VALUES ($1, $2, $3)")
        .bind(fingerprint(&token))
        .bind(time::OffsetDateTime::now_utc() + SESSION_TTL)
        .bind(participant)
        .execute(db)
        .await?;
    Ok(token)
}

/// Preuve qu'une requête émane d'un élève authentifié.
///
/// Porte l'identifiant du participant et celui de son groupe : toute
/// vérification d'appartenance part de là, jamais d'un champ fourni par le
/// client (SPEC §17).
pub struct Participant {
    pub id: Uuid,
    pub group_id: Uuid,
}

impl FromRequestParts<AppState> for Participant {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = session_cookie(parts).ok_or(AppError::Unauthorized)?;

        let row: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT p.id, p.group_id
               FROM sessions s
               JOIN participants p ON p.id = s.participant_id
              WHERE s.token_hash = $1
                AND s.expires_at > now()
                AND p.active
                AND p.expires_at > now()",
        )
        .bind(fingerprint(&token))
        .fetch_optional(&state.db)
        .await?;

        row.map(|(id, group_id)| Participant { id, group_id })
            .ok_or(AppError::Unauthorized)
    }
}

/// Preuve qu'une requête émane d'un principal autorisé à **rédiger** un sujet.
///
/// L'enseignant et l'agent y sont tous deux admis : déposer un brouillon est
/// précisément ce qu'un agent doit pouvoir faire. En revanche la validation,
/// l'ouverture d'une évaluation et tout ce qui détruit restent réservés à
/// l'extracteur [`Teacher`] — c'est la règle « l'agent écrit, l'humain
/// publie » de [ADR-0003](../../docs/adr/0003-administration-par-facade-mcp.md),
/// portée par le type plutôt que par la vigilance du handler.
pub enum Author {
    Teacher,
    Agent,
}

impl Author {
    pub fn as_str(&self) -> &'static str {
        match self {
            Author::Teacher => "teacher",
            Author::Agent => "agent",
        }
    }
}

impl FromRequestParts<AppState> for Author {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if Teacher::from_request_parts(parts, state).await.is_ok() {
            return Ok(Author::Teacher);
        }
        Agent::from_request_parts(parts, state)
            .await
            .map(|_| Author::Agent)
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

/// Détermine l'adresse du client derrière un reverse proxy.
///
/// Sans cette étape, dans la topologie de production (Nginx en frontal), toutes
/// les requêtes porteraient l'adresse du proxy. La limitation par IP
/// deviendrait une limitation globale, et n'importe qui pourrait verrouiller
/// l'enseignant hors de son instance en épuisant le quota — exactement ce que
/// le comptage par IP visait à empêcher.
///
/// `X-Forwarded-For` n'est lu **que si le pair est un proxy déclaré de
/// confiance**. Un en-tête est trivial à forger : le lire inconditionnellement
/// permettrait à un attaquant d'annoncer une adresse différente à chaque
/// tentative et de contourner entièrement la limitation.
///
/// La valeur retenue est la **dernière** de la liste : c'est celle que le proxy
/// de confiance vient d'ajouter. Les valeurs précédentes peuvent avoir été
/// forgées par le client.
pub fn client_ip(peer: IpAddr, headers: &axum::http::HeaderMap, trusted: &[ipnet::IpNet]) -> IpAddr {
    if !trusted.iter().any(|net| net.contains(&peer)) {
        return peer;
    }
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(peer)
}

/// Fenêtre fixe par adresse IP sur l'endpoint de connexion (SPEC §17).
///
/// L'adresse est celle rendue par [`client_ip`], donc l'adresse réelle du
/// client et non celle du reverse proxy.
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

    fn headers(xff: Option<&str>) -> axum::http::HeaderMap {
        let mut h = axum::http::HeaderMap::new();
        if let Some(v) = xff {
            h.insert("x-forwarded-for", v.parse().unwrap());
        }
        h
    }

    fn cidr(s: &str) -> Vec<ipnet::IpNet> {
        vec![s.parse().unwrap()]
    }

    #[test]
    fn sans_proxy_de_confiance_len_tete_est_ignore() {
        let peer: IpAddr = "203.0.113.9".parse().unwrap();
        assert_eq!(
            client_ip(peer, &headers(Some("1.2.3.4")), &[]),
            peer,
            "un client non déclaré de confiance ne doit pas pouvoir annoncer son IP"
        );
    }

    #[test]
    fn proxy_de_confiance_len_tete_est_lu() {
        let peer: IpAddr = "172.18.0.1".parse().unwrap();
        assert_eq!(
            client_ip(peer, &headers(Some("198.51.100.7")), &cidr("172.16.0.0/12")),
            "198.51.100.7".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn chaine_forgee_la_derniere_valeur_prime() {
        // Le client envoie « 1.2.3.4 », Nginx ajoute son propre pair derrière.
        let peer: IpAddr = "172.18.0.1".parse().unwrap();
        assert_eq!(
            client_ip(peer, &headers(Some("1.2.3.4, 198.51.100.7")), &cidr("172.16.0.0/12")),
            "198.51.100.7".parse::<IpAddr>().unwrap(),
            "la valeur ajoutée par le proxy de confiance doit primer sur la valeur forgée"
        );
    }

    #[test]
    fn en_tete_absent_ou_illisible_retombe_sur_le_pair() {
        let peer: IpAddr = "172.18.0.1".parse().unwrap();
        let trusted = cidr("172.16.0.0/12");
        assert_eq!(client_ip(peer, &headers(None), &trusted), peer);
        assert_eq!(client_ip(peer, &headers(Some("pas-une-ip")), &trusted), peer);
        assert_eq!(client_ip(peer, &headers(Some("")), &trusted), peer);
    }

    #[test]
    fn scope_aller_retour() {
        assert_eq!(Scope::parse("agent"), Some(Scope::Agent));
        assert_eq!(Scope::parse("full"), None);
        assert_eq!(Scope::parse(Scope::Agent.as_str()), Some(Scope::Agent));
    }
}
