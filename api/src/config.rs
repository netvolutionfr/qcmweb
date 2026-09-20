use std::env;

/// Configuration lue depuis l'environnement au démarrage.
///
/// Toute valeur manquante est une erreur fatale : on refuse de démarrer à
/// moitié configuré plutôt que de découvrir le problème à la première requête.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub auth: AuthConfig,
}

/// Identifiants de l'unique enseignant de l'instance.
///
/// Le mot de passe est fixé : ni inscription, ni réinitialisation en
/// libre-service. Les passkeys remplaceront ce mécanisme ultérieurement.
#[derive(Clone)]
pub struct AuthConfig {
    pub username: String,
    pub password_hash: String,
}

// Écrit à la main : un `derive(Debug)` ferait fuir l'empreinte du mot de passe
// dans les journaux au premier `tracing::debug!(?config)`.
impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("username", &self.username)
            .field("password_hash", &"<masqué>")
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into()),
            auth: AuthConfig {
                username: required("TEACHER_USERNAME")?,
                password_hash: required("TEACHER_PASSWORD_HASH")?,
            },
        })
    }
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
