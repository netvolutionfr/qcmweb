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
use std::hash::Hash;
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

/// Empreinte des identifiants de l'enseignant.
///
/// Portée par chaque session d'enseignant. Changer le mot de passe ou
/// l'identifiant change cette empreinte, et **toutes les sessions ouvertes
/// jusque-là cessent d'être valides** : une session volée ne survit pas à la
/// rotation du credential qu'elle a servi à obtenir.
pub fn credential_fingerprint(username: &str, password_hash: &str) -> String {
    // Le `\0` empêche qu'un déplacement de frontière entre les deux champs
    // produise la même empreinte.
    fingerprint(&format!("{username}\0{password_hash}"))
}

/// Crée une session d'enseignant et renvoie le jeton en clair — la seule et
/// unique fois.
pub async fn open_session(db: &PgPool, credential: &str) -> Result<String, AppError> {
    let token = random_secret();
    sqlx::query("INSERT INTO sessions (token_hash, expires_at, credential) VALUES ($1, $2, $3)")
        .bind(fingerprint(&token))
        .bind(time::OffsetDateTime::now_utc() + SESSION_TTL)
        .bind(credential)
        .execute(db)
        .await?;
    Ok(token)
}

/// Supprime les sessions d'enseignant ouvertes sous d'anciens identifiants.
///
/// La validation les refuse déjà ; ceci évite de garder des lignes mortes trente
/// jours après une rotation.
pub async fn purge_stale_teacher_sessions(db: &PgPool, credential: &str) -> Result<u64, AppError> {
    let done = sqlx::query(
        "DELETE FROM sessions WHERE participant_id IS NULL AND credential IS DISTINCT FROM $1",
    )
    .bind(credential)
    .execute(db)
    .await?;
    Ok(done.rows_affected())
}

/// Vérifie un mot de passe **hors de l'exécuteur asynchrone**, à concurrence
/// bornée.
///
/// Argon2id occupe un cœur et une vingtaine de mégaoctets pendant la
/// vérification. Dans un handler, il bloquerait un worker Tokio et gèlerait les
/// autres requêtes ; sans borne, un flot de connexions ouvrirait autant de
/// vérifications que de threads disponibles. Le sémaphore plafonne les deux.
pub async fn verify_blocking(
    state: &AppState,
    password: &str,
    phc: &str,
) -> Result<bool, AppError> {
    let _permit = state
        .hashing
        .acquire()
        .await
        .map_err(|_| AppError::Internal("file de hachage fermée"))?;

    let (password, phc) = (password.to_owned(), phc.to_owned());
    tokio::task::spawn_blocking(move || verify_password(&password, &phc))
        .await
        .map_err(|_| AppError::Internal("vérification interrompue"))?
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
        //
        // `credential = $2` lie la session aux identifiants en vigueur : après un
        // changement de mot de passe, les sessions d'avant sont refusées.
        let valid: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM sessions
              WHERE token_hash = $1 AND expires_at > now()
                AND participant_id IS NULL AND credential = $2",
        )
        .bind(fingerprint(&token))
        .bind(&state.auth.credential)
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

/// Compteur de tentatives par clé, à fenêtre fixe (SPEC §17, ADR-0015).
///
/// **La clé est ce que l'on cherche à deviner.** C'est le point qui compte : une
/// réussite n'autorise à remettre à zéro que le compteur de la chose dont elle
/// prouve la connaissance. Un compteur unique remis à zéro par n'importe quelle
/// connexion réussie permettait à un élève, connecté sous son propre compte,
/// d'effacer le quota qui protégeait le mot de passe de l'enseignant.
///
/// Le quota est **réservé avant** la vérification ([`Throttle::admit`]). Compter
/// après laisserait passer autant d'essais que de requêtes simultanées.
///
/// ponytail: compteurs en mémoire, remis à zéro au redémarrage et non partagés
/// entre instances. Suffisant pour une instance unique ; passer à Postgres ou
/// Redis le jour où il y en a plusieurs.
pub struct Throttle<K> {
    hits: Mutex<HashMap<K, (u32, Instant)>>,
    max: u32,
    window: Duration,
}

impl<K: Eq + Hash + Clone> Throttle<K> {
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            hits: Mutex::new(HashMap::new()),
            max,
            window,
        }
    }

    /// Réserve une tentative. `false` si le quota de la clé est épuisé.
    ///
    /// Un refus ne prolonge pas la fenêtre : marteler un compteur déjà plein ne
    /// doit pas repousser son déblocage.
    pub fn admit(&self, key: &K) -> bool {
        self.admit_at(key, Instant::now())
    }

    fn admit_at(&self, key: &K, now: Instant) -> bool {
        let mut hits = self.hits.lock().expect("mutex empoisonné");
        // Purge opportuniste : évite que la table enfle sous une attaque
        // distribuée. ponytail: O(n) à chaque appel, négligeable tant que le
        // trafic de connexion reste celui d'une classe.
        hits.retain(|_, (_, start)| now.duration_since(*start) < self.window);

        let entry = hits.entry(key.clone()).or_insert((0, now));
        if entry.0 >= self.max {
            return false;
        }
        entry.0 += 1;
        true
    }

    /// Le quota de la clé est-il épuisé ? Ne modifie rien.
    pub fn blocked(&self, key: &K) -> bool {
        let now = Instant::now();
        self.hits
            .lock()
            .expect("mutex empoisonné")
            .get(key)
            .is_some_and(|(n, start)| now.duration_since(*start) < self.window && *n >= self.max)
    }

    /// Enregistre une tentative sans en vérifier le quota.
    ///
    /// Pour les compteurs qui ne comptent que les échecs et se consultent avant
    /// (`blocked`) : ils n'ont pas de « réussite » qui les libère.
    pub fn record(&self, key: K) {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("mutex empoisonné");
        hits.retain(|_, (_, start)| now.duration_since(*start) < self.window);
        let entry = hits.entry(key).or_insert((0, now));
        entry.0 = entry.0.saturating_add(1);
    }

    /// Remet à zéro la clé. À n'appeler que si la réussite **prouve la
    /// connaissance du secret de cette clé** : jamais sur un compteur partagé
    /// avec d'autres comptes.
    pub fn clear(&self, key: &K) {
        self.hits.lock().expect("mutex empoisonné").remove(key);
    }
}

/// Les compteurs de connexion, de types volontairement distincts.
///
/// Les clés n'ont pas le même type : impossible de remettre à zéro le quota de
/// l'enseignant depuis le parcours élève, ou l'inverse. Le partage qui rendait le
/// contournement possible n'est plus exprimable.
///
/// Il n'y a volontairement **pas** de compteur sur les jetons inconnus. Un jeton
/// fait environ 39 bits pour une trentaine de valides : l'énumérer est hors de
/// portée, et un jeton identifie sans authentifier. En revanche un tel compteur,
/// par adresse, aurait permis à un seul élève de verrouiller toute la classe
/// derrière la même adresse hors de l'épreuve (ADR-0015).
pub struct Throttles {
    /// Essais sur le mot de passe de l'enseignant, par adresse.
    ///
    /// Par adresse et non global : un tiers ne doit pas pouvoir verrouiller
    /// l'enseignant hors de son propre outil.
    pub teacher: Throttle<IpAddr>,
    /// Essais sur le secret d'un participant, par (jeton, adresse).
    ///
    /// Par jeton : ce que l'on protège est ce secret-là, et seule sa
    /// connaissance permet de libérer son compteur. Avec l'adresse : un tiers
    /// qui connaît un jeton ne peut pas en verrouiller le propriétaire depuis
    /// un autre poste.
    pub student: Throttle<(String, IpAddr)>,
}

impl Throttles {
    pub fn new() -> Self {
        let window = Duration::from_secs(15 * 60);
        Self {
            teacher: Throttle::new(10, window),
            student: Throttle::new(10, window),
        }
    }
}

impl Default for Throttles {
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

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn quinze_minutes() -> Duration {
        Duration::from_secs(15 * 60)
    }

    #[test]
    fn throttle_bloque_apres_le_quota() {
        let t = Throttle::new(10, quinze_minutes());
        let k = ip("192.0.2.1");
        for i in 1..=10 {
            assert!(t.admit(&k), "tentative {i} devrait passer");
        }
        assert!(!t.admit(&k), "la 11e doit être refusée");
    }

    #[test]
    fn throttle_isole_les_cles() {
        let t = Throttle::new(10, quinze_minutes());
        let attaquant = ip("192.0.2.1");
        let enseignant = ip("192.0.2.2");
        for _ in 0..20 {
            t.admit(&attaquant);
        }
        assert!(
            t.admit(&enseignant),
            "l'enseignant ne doit pas être verrouillé par un tiers"
        );
    }

    /// Le contournement d'origine : un élève réussissait sa propre connexion et
    /// effaçait le quota qui protégeait un autre compte.
    #[test]
    fn reussir_sur_une_cle_ne_libere_pas_une_autre() {
        let t: Throttle<(String, IpAddr)> = Throttle::new(10, quinze_minutes());
        let poste = ip("192.0.2.7");
        let victime = ("VICTIME1".to_string(), poste);
        let attaquant = ("ATTAQUANT".to_string(), poste);

        for _ in 0..10 {
            t.admit(&victime);
        }
        assert!(t.blocked(&victime));

        // L'attaquant se connecte à son propre compte depuis la même adresse.
        t.admit(&attaquant);
        t.clear(&attaquant);

        assert!(
            t.blocked(&victime),
            "la connexion de l'attaquant ne doit pas libérer le compteur de la victime"
        );
        assert!(!t.admit(&victime));
    }

    #[test]
    fn clear_libere_sa_propre_cle() {
        let t = Throttle::new(10, quinze_minutes());
        let k = ip("192.0.2.3");
        for _ in 0..10 {
            t.admit(&k);
        }
        t.clear(&k);
        assert!(t.admit(&k), "réussir sur sa propre clé la libère");
    }

    #[test]
    fn blocked_ne_consomme_rien() {
        let t = Throttle::new(3, quinze_minutes());
        let k = ip("192.0.2.4");
        for _ in 0..100 {
            assert!(!t.blocked(&k));
        }
        assert!(t.admit(&k) && t.admit(&k) && t.admit(&k), "le quota est intact");
    }

    #[test]
    fn record_puis_blocked() {
        let t = Throttle::new(3, quinze_minutes());
        let k = ip("192.0.2.5");
        for _ in 0..2 {
            t.record(k);
        }
        assert!(!t.blocked(&k));
        t.record(k);
        assert!(t.blocked(&k));
    }

    #[test]
    fn la_fenetre_expire_et_un_refus_ne_la_prolonge_pas() {
        let t = Throttle::new(2, Duration::from_secs(60));
        let k = ip("192.0.2.6");
        let t0 = Instant::now();

        assert!(t.admit_at(&k, t0));
        assert!(t.admit_at(&k, t0 + Duration::from_secs(10)));
        assert!(!t.admit_at(&k, t0 + Duration::from_secs(30)));
        // Un refus tardif ne repousse pas la libération.
        assert!(!t.admit_at(&k, t0 + Duration::from_secs(59)));
        assert!(
            t.admit_at(&k, t0 + Duration::from_secs(60)),
            "la fenêtre court depuis la première tentative"
        );
    }

    /// Le quota est réservé avant la vérification : cent requêtes simultanées
    /// ne doivent pas obtenir plus d'essais que le quota.
    #[test]
    fn reservation_atomique_sous_concurrence() {
        use std::sync::Arc;
        let t = Arc::new(Throttle::new(10, quinze_minutes()));
        let k = ip("192.0.2.9");

        let admis: usize = (0..100)
            .map(|_| {
                let t = Arc::clone(&t);
                std::thread::spawn(move || t.admit(&k))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap() as usize)
            .sum();

        assert_eq!(admis, 10);
    }

    #[test]
    fn empreinte_d_identifiants_sensible_a_chaque_champ() {
        let base = credential_fingerprint("enseignant", "$argon2id$hash-a");
        assert_eq!(base, credential_fingerprint("enseignant", "$argon2id$hash-a"));
        assert_ne!(base, credential_fingerprint("enseignant", "$argon2id$hash-b"));
        assert_ne!(base, credential_fingerprint("autre", "$argon2id$hash-a"));
        // Le séparateur interdit qu'un déplacement de frontière collisionne.
        assert_ne!(credential_fingerprint("ab", "c"), credential_fingerprint("a", "bc"));
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
