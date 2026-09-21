use std::env;

use ipnet::IpNet;

/// Configuration lue depuis l'environnement au démarrage.
///
/// Toute valeur manquante est une erreur fatale : on refuse de démarrer à
/// moitié configuré plutôt que de découvrir le problème à la première requête.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub auth: AuthConfig,
    /// Réseaux depuis lesquels `X-Forwarded-For` est digne de foi.
    ///
    /// Vide par défaut : sans configuration explicite, l'application ne croit
    /// personne et s'en tient à l'adresse du pair. Un défaut permissif serait
    /// une faille silencieuse en développement comme en production.
    pub trusted_proxies: Vec<IpNet>,
}

/// Identifiants de l'unique enseignant de l'instance.
///
/// Le mot de passe est fixé : ni inscription, ni réinitialisation en
/// libre-service. Les passkeys remplaceront ce mécanisme ultérieurement.
#[derive(Clone)]
pub struct AuthConfig {
    pub username: String,
    pub password_hash: String,
    /// Empreinte des identifiants ci-dessus, portée par les sessions ouvertes.
    pub credential: String,
}

// Écrit à la main : un `derive(Debug)` ferait fuir l'empreinte du mot de passe
// dans les journaux au premier `tracing::debug!(?config)`.
impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("username", &self.username)
            .field("password_hash", &"<masqué>")
            .field("credential", &"<masqué>")
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into()),
            auth: {
                let username = required("TEACHER_USERNAME")?;
                let password_hash = required("TEACHER_PASSWORD_HASH")?;
                let credential = crate::auth::credential_fingerprint(&username, &password_hash);
                AuthConfig {
                    username,
                    password_hash,
                    credential,
                }
            },
            trusted_proxies: trusted_proxies()?,
        })
    }
}

fn trusted_proxies() -> Result<Vec<IpNet>, String> {
    let raw = env::var("TRUSTED_PROXY_CIDRS").unwrap_or_default();
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<IpNet>()
                .map_err(|e| format!("TRUSTED_PROXY_CIDRS : `{s}` invalide ({e})"))
        })
        .collect()
}

fn required(key: &str) -> Result<String, String> {
    env::var(key)
        .map_err(|_| format!("variable d'environnement manquante : {key}"))
        .and_then(|v| {
            if v.trim().is_empty() {
                Err(format!("variable d'environnement vide : {key}"))
            } else {
                Ok(v)
            }
        })
}
