use std::env;

/// Configuration lue depuis l'environnement au démarrage.
///
/// Toute valeur manquante est une erreur fatale : on refuse de démarrer à
/// moitié configuré plutôt que de découvrir le problème à la première requête.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into()),
        })
    }
}

fn required(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("variable d'environnement manquante : {key}"))
}
