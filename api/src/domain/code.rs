//! Génération des codes tapés par un humain : jetons de participation, secrets,
//! codes d'évaluation.
//!
//! La contrainte dominante n'est pas cryptographique mais ergonomique : un élève
//! recopie ces codes depuis un billet papier, parfois sur un téléphone. Tout
//! caractère prêtant à confusion est une tentative de connexion perdue.

use rand::Rng;

/// Alphabet sans caractères ambigus : ni `0`/`O`, ni `1`/`I`/`L`.
///
/// `U` est également exclu, suivant la convention Crockford : cela réduit la
/// probabilité qu'un tirage aléatoire produise un mot malvenu sur un document
/// distribué à des élèves.
const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Le plus grand multiple de 30 tenant dans un octet.
///
/// Un simple `octet % 30` favoriserait les premiers caractères de l'alphabet,
/// puisque 256 n'est pas divisible par 30. On rejette donc la queue de la
/// distribution : le biais est nul et le coût est de 6 tirages sur 256.
const REJECT_AT: u8 = 240;

fn code(len: usize) -> String {
    let mut rng = rand::rng();
    let mut out = String::with_capacity(len);
    let mut buf = [0u8; 32];

    while out.len() < len {
        rng.fill_bytes(&mut buf);
        for &b in &buf {
            if b < REJECT_AT && out.len() < len {
                out.push(ALPHABET[(b % ALPHABET.len() as u8) as usize] as char);
            }
        }
    }
    out
}

fn grouped(raw: &str, by: usize) -> String {
    raw.as_bytes()
        .chunks(by)
        .map(|c| std::str::from_utf8(c).expect("alphabet ASCII"))
        .collect::<Vec<_>>()
        .join("-")
}

/// Jeton de participation : 8 caractères, ~39 bits.
///
/// Il identifie, il n'authentifie pas — le secret s'en charge. Cette entropie
/// suffit à rendre l'énumération inutile.
pub fn participant_token() -> String {
    grouped(&code(8), 4)
}

/// Secret d'un participant : 10 caractères, ~49 bits.
///
/// Recopié à la main, donc court, donc haché en Argon2id : c'est exactement le
/// cas d'un secret à entropie limitée que le coût du KDF vient compenser.
/// Voir [ADR-0005](../../../docs/adr/0005-cle-api-et-choix-du-hachage.md).
pub fn participant_secret() -> String {
    grouped(&code(10), 5)
}

/// Code d'évaluation : 6 caractères, ~29 bits.
///
/// Il est annoncé à voix haute ou projeté, donc court. Il ne protège rien par
/// lui-même (SPEC §11) : il désigne une évaluation ouverte, et l'appartenance
/// au groupe reste vérifiée côté serveur.
pub fn assessment_code() -> String {
    code(6)
}

/// Ramène un code à sa **forme canonique** : majuscules, sans séparateur.
///
/// C'est cette forme qui est hachée et comparée, jamais la forme affichée. Le
/// tiret n'existe que pour la lisibilité du billet papier ; hacher la forme
/// affichée rendrait toute saisie normalisée invalide.
///
/// Un élève tape en minuscules, oublie le tiret, en ajoute un au mauvais
/// endroit, ou colle une espace insécable. Refuser ces saisies serait une
/// mauvaise manière de faire respecter un format que nous avons choisi.
pub fn normalize(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_attendus() {
        let token = participant_token();
        assert_eq!(token.len(), 9, "8 caractères et un tiret : {token}");
        assert_eq!(token.chars().filter(|c| *c == '-').count(), 1);

        let secret = participant_secret();
        assert_eq!(secret.len(), 11, "10 caractères et un tiret : {secret}");

        assert_eq!(assessment_code().len(), 6);
    }

    #[test]
    fn aucun_caractere_ambigu() {
        // Volume suffisant pour que l'absence soit significative et non chanceuse.
        for _ in 0..500 {
            for c in participant_token().chars().chain(participant_secret().chars()) {
                assert!(
                    !"01OIL U".contains(c),
                    "caractère ambigu `{c}` dans un code destiné à être recopié"
                );
            }
        }
    }

    #[test]
    fn codes_non_repetes() {
        let mut vus = std::collections::HashSet::new();
        for _ in 0..1000 {
            assert!(vus.insert(participant_token()), "collision sur 1000 tirages");
        }
    }

    #[test]
    fn distribution_sans_biais_grossier() {
        // Le rejet de la queue doit produire une distribution plate. Un simple
        // `% 30` ferait sortir les 16 premiers caractères environ 7 % plus
        // souvent : on vérifie que l'écart reste bien en deçà.
        let mut counts = std::collections::HashMap::new();
        for _ in 0..2000 {
            for c in code(10).chars() {
                *counts.entry(c).or_insert(0usize) += 1;
            }
        }
        assert_eq!(counts.len(), ALPHABET.len(), "tous les caractères tirés");

        let attendu = 20_000.0 / ALPHABET.len() as f64;
        for (c, n) in &counts {
            let ecart = (*n as f64 - attendu).abs() / attendu;
            assert!(ecart < 0.15, "`{c}` tiré {n} fois, attendu ~{attendu:.0}");
        }
    }

    #[test]
    fn la_forme_affichee_et_la_forme_canonique_correspondent() {
        let secret = participant_secret();
        assert!(secret.contains('-'), "forme affichée : {secret}");
        assert_eq!(
            normalize(&secret).len(),
            10,
            "la forme canonique perd le tiret, pas les caractères"
        );
        assert_eq!(normalize(&secret), normalize(&secret.to_lowercase()));
    }

    #[test]
    fn normalisation_des_saisies() {
        let canonique = "ABCD2345";
        for saisie in [
            "ABCD-2345",
            "abcd-2345",
            "abcd2345",
            " ABCD 2345 ",
            "AB-CD-23-45",
            "abcd–2345", // tiret cadratin collé par un traitement de texte
        ] {
            assert_eq!(normalize(saisie), canonique, "saisie : {saisie:?}");
        }
    }
}
