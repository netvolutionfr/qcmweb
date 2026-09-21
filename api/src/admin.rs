//! Sous-commandes d'administration.
//!
//! Il n'existe ni inscription, ni réinitialisation en libre-service : le mot de
//! passe de l'enseignant est fixé, et les clés d'API se créent ici. C'est
//! volontaire tant que l'instance sert un seul enseignant.

use argon2::{Argon2, PasswordHasher};

use crate::auth::{mint_api_key, Scope};
use crate::config::Config;

/// Longueur minimale d'un mot de passe enseignant, en caractères.
///
/// La politique ne s'applique **qu'ici**, à la création du mot de passe humain.
/// `hash_password` sert aussi à hacher les secrets de participants, plus courts
/// par conception (ADR-0007) : y mettre la règle les rendrait tous invalides.
pub const MIN_PASSWORD_CHARS: usize = 12;

/// Refuse les mots de passe trop faibles pour un compte exposé sur Internet.
///
/// Compte des caractères et non des octets, sans quoi « é » vaudrait deux.
/// Le nombre de caractères distincts écarte `aaaaaaaaaaaa` et douze espaces,
/// qui satisfont la longueur sans rien protéger.
pub fn check_password_policy(password: &str) -> Result<(), String> {
    let length = password.chars().count();
    if length < MIN_PASSWORD_CHARS {
        return Err(format!(
            "mot de passe trop court : {MIN_PASSWORD_CHARS} caractères minimum, {length} saisis. \
             Une phrase de passe de quelques mots convient très bien."
        ));
    }
    let distinct: std::collections::HashSet<char> = password.chars().collect();
    if distinct.len() < 5 {
        return Err("mot de passe trop répétitif : moins de cinq caractères différents".into());
    }
    Ok(())
}

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

/// Lit le mot de passe sans jamais l'afficher, et sans qu'il passe par les
/// arguments de la commande : il atterrirait dans l'historique du shell et dans
/// la table des processus.
///
/// Au terminal, la saisie est masquée et demandée deux fois — une faute de frappe
/// sur un mot de passe que l'on ne voit pas enfermerait dehors. Depuis un tube,
/// la première ligne est lue telle quelle, ce qui permet de scripter.
fn read_new_password() -> Result<String, Box<dyn std::error::Error>> {
    use std::io::IsTerminal;

    if std::io::stdin().is_terminal() {
        let first = rpassword::prompt_password("Mot de passe : ")?;
        let second = rpassword::prompt_password("Confirmation : ")?;
        if first != second {
            return Err("les deux saisies diffèrent".into());
        }
        Ok(first)
    } else {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(line.trim_end_matches(['\n', '\r']).to_string())
    }
}

pub async fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args[0].as_str() {
        "hash-password" => {
            let password = read_new_password()?;
            check_password_policy(&password)?;

            println!("{}", hash_password(&password)?);
            eprintln!("\nÀ placer dans .env :\n  TEACHER_PASSWORD_HASH='<la ligne ci-dessus>'");
            eprintln!(
                "\nChanger le mot de passe révoque les sessions ouvertes au prochain démarrage \
                 de l'API."
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longueur_minimale() {
        assert!(check_password_policy("abcdefghijk").is_err(), "11 caractères");
        assert!(check_password_policy("abcdefghijkl").is_ok(), "12 caractères");
    }

    #[test]
    fn la_longueur_compte_des_caracteres_pas_des_octets() {
        // Douze caractères distincts de deux octets chacun : 24 octets.
        let douze = "éàùçêôîâëïüœ";
        assert_eq!(douze.chars().count(), 12);
        assert!(check_password_policy(douze).is_ok());

        // Onze caractères de deux octets : 22 octets, donc plus que le minimum
        // si l'on comptait des octets, mais trop court en caractères.
        let onze = "éàùçêôîâëïü";
        assert_eq!(onze.chars().count(), 11);
        assert!(check_password_policy(onze).is_err());
    }

    #[test]
    fn refuse_le_repetitif_meme_long() {
        assert!(check_password_policy("aaaaaaaaaaaaaaaa").is_err());
        assert!(check_password_policy("            ").is_err(), "douze espaces");
        assert!(check_password_policy("abababababab").is_err());
    }

    #[test]
    fn accepte_une_phrase_de_passe() {
        assert!(check_password_policy("cheval agrafe pile").is_ok());
    }

    /// `hash_password` reste utilisable sur des secrets courts : la politique
    /// n'y est pas, c'est voulu.
    #[test]
    fn hash_password_n_impose_pas_la_politique() {
        assert!(hash_password("PMQX47ME4E").is_ok());
    }
}
