//! Sous-commandes d'administration.
//!
//! Il n'existe ni inscription, ni réinitialisation en libre-service : le mot de
//! passe de l'enseignant est fixé, et les clés d'API se créent ici. C'est
//! volontaire tant que l'instance sert un seul enseignant.

use argon2::{Argon2, PasswordHasher};

use crate::auth::{mint_api_key, Scope};
use crate::config::Config;

/// Empreinte Argon2id au format PHC, à coller dans `TEACHER_PASSWORD_HASH`.
pub fn hash_password(password: &str) -> Result<String, String> {
    // argon2 0.6 tire lui-même le sel depuis le générateur du système.
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| format!("hachage impossible : {e}"))
}

pub const USAGE: &str = "\
Usage :
  qcmweb-api                          démarre l'API
  qcmweb-api hash-password            empreinte un mot de passe (saisie interactive)
  qcmweb-api mint-key <libellé>       crée une clé d'API de scope `agent`
";

pub async fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args[0].as_str() {
        "hash-password" => {
            // Lu sur l'entrée standard plutôt qu'en argument : un mot de passe
            // passé en argument atterrit dans l'historique du shell et dans la
            // table des processus.
            eprint!("Mot de passe : ");
            let mut password = String::new();
            std::io::stdin().read_line(&mut password)?;
            let password = password.trim_end_matches(['\n', '\r']);

            if password.is_empty() {
                return Err("mot de passe vide".into());
            }
            println!("{}", hash_password(password)?);
            eprintln!("\nÀ placer dans .env :\n  TEACHER_PASSWORD_HASH='<la ligne ci-dessus>'");
        }

        "mint-key" => {
            let label = args.get(1).ok_or("libellé manquant : mint-key <libellé>")?;
            let config = Config::from_env()?;
            let db = sqlx::PgPool::connect(&config.database_url).await?;

            let key = mint_api_key(&db, label, Scope::Agent).await?;
            println!("{key}");
            eprintln!(
                "\nClé de scope `agent` créée pour « {label} ».\n\
                 Elle ne sera plus jamais affichée.\n\n\
                 À placer dans la configuration de ton client MCP, jamais dans une conversation :\n  \
                 Authorization: Bearer <la ligne ci-dessus>"
            );
        }

        other => return Err(format!("sous-commande inconnue : {other}\n\n{USAGE}").into()),
    }
    Ok(())
}
