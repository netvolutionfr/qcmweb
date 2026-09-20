//! Règles d'une évaluation, hors persistance.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum Mode {
    /// Sans enjeu de notation : le retour immédiat a du sens.
    Formative,
    /// Le résultat pourra devenir une note ; toute retouche est tracée.
    Summative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Release {
    Immediate,
    AfterClose,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum State {
    /// Créée, jamais ouverte. Le code existe mais ne donne accès à rien.
    Draft,
    Open,
    Closed,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Draft => "DRAFT",
            State::Open => "OPEN",
            State::Closed => "CLOSED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "DRAFT" => Some(State::Draft),
            "OPEN" => Some(State::Open),
            "CLOSED" => Some(State::Closed),
            _ => None,
        }
    }
}

/// Une évaluation est-elle composable à cet instant ?
///
/// Deux conditions cumulatives, volontairement distinctes :
///
/// - l'enseignant l'a **ouverte** ;
/// - l'instant est dans la **fenêtre planifiée**, si elle est renseignée.
///
/// Séparer les deux permet de préparer une évaluation à l'avance sans qu'elle
/// devienne accessible, et de la refermer d'un geste sans toucher aux dates.
pub fn is_available(
    state: State,
    opens_at: Option<OffsetDateTime>,
    closes_at: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> bool {
    if state != State::Open {
        return false;
    }
    if opens_at.is_some_and(|start| now < start) {
        return false;
    }
    if closes_at.is_some_and(|end| now >= end) {
        return false;
    }
    true
}

/// Le score est-il divulgable ?
pub fn releases(rule: Release, available: bool) -> bool {
    match rule {
        Release::Immediate => true,
        // « Après fermeture » signifie : tant que l'épreuve est composable, un
        // élève rapide ne doit rien pouvoir transmettre aux autres (SPEC §10).
        Release::AfterClose => !available,
        Release::Never => false,
    }
}

/// Conversion du score en note.
pub fn grade(score: f64, max_score: f64, max_grade: f64) -> Option<f64> {
    (max_score > 0.0).then(|| score / max_score * max_grade)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn t(offset_minutes: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::minutes(offset_minutes)
    }

    #[test]
    fn fermee_ou_brouillon_jamais_disponible() {
        for state in [State::Draft, State::Closed] {
            assert!(!is_available(state, None, None, t(0)));
        }
    }

    #[test]
    fn ouverte_sans_fenetre_est_disponible() {
        assert!(is_available(State::Open, None, None, t(0)));
    }

    #[test]
    fn avant_ouverture_planifiee() {
        assert!(!is_available(State::Open, Some(t(10)), None, t(5)));
        assert!(is_available(State::Open, Some(t(10)), None, t(10)));
    }

    #[test]
    fn apres_fermeture_planifiee() {
        assert!(is_available(State::Open, None, Some(t(10)), t(9)));
        assert!(
            !is_available(State::Open, None, Some(t(10)), t(10)),
            "la borne de fermeture est exclue : à l'heure dite, c'est fini"
        );
    }

    #[test]
    fn ouverture_manuelle_ne_suffit_pas_hors_fenetre() {
        assert!(!is_available(State::Open, Some(t(0)), Some(t(10)), t(20)));
    }

    #[test]
    fn divulgation_du_score() {
        assert!(releases(Release::Immediate, true));
        assert!(releases(Release::Immediate, false));
        assert!(
            !releases(Release::AfterClose, true),
            "pendant l'épreuve, un élève rapide ne doit rien pouvoir transmettre"
        );
        assert!(releases(Release::AfterClose, false));
        assert!(!releases(Release::Never, false));
    }

    #[test]
    fn conversion_en_note() {
        assert_eq!(grade(17.0, 20.0, 20.0), Some(17.0));
        assert_eq!(grade(4.0, 8.0, 20.0), Some(10.0));
        assert_eq!(grade(3.0, 4.0, 10.0), Some(7.5));
        assert_eq!(grade(1.0, 0.0, 20.0), None, "barème nul : pas de note");
    }

    #[test]
    fn etat_aller_retour() {
        for state in [State::Draft, State::Open, State::Closed] {
            assert_eq!(State::parse(state.as_str()), Some(state));
        }
        assert_eq!(State::parse("ARCHIVED"), None);
    }
}
