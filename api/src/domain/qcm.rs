//! Modèle natif `qcm/v1`.
//!
//! Ce module ne dépend d'aucun format externe (AMC-TXT, Moodle XML, GIFT) :
//! ceux-ci seront des adaptateurs en entrée. Il ne dépend pas davantage de la
//! base ni du transport — c'est du domaine pur, donc testable sans rien monter.

use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "qcm/v1";

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Document {
    pub schema: String,
    pub metadata: Metadata,
    pub questions: Vec<Question>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Metadata {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Un enum plutôt qu'un `kind: String` accompagné de champs optionnels : les
/// états incohérents (un `true_false` porteur de propositions, un
/// `single_choice` doté d'un mode de notation partielle) sont inexprimables.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Question {
    SingleChoice(SingleChoice),
    MultipleChoice(MultipleChoice),
    TrueFalse(TrueFalse),
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SingleChoice {
    pub id: String,
    pub prompt: String,
    pub choices: Vec<Choice>,
    #[serde(default = "one")]
    pub points: f64,
    #[serde(flatten)]
    pub teaching: Teaching,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MultipleChoice {
    pub id: String,
    pub prompt: String,
    pub choices: Vec<Choice>,
    #[serde(default = "one")]
    pub points: f64,
    #[serde(default)]
    pub scoring: Scoring,
    #[serde(flatten)]
    pub teaching: Teaching,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TrueFalse {
    pub id: String,
    pub prompt: String,
    pub answer: bool,
    #[serde(default = "one")]
    pub points: f64,
    #[serde(flatten)]
    pub teaching: Teaching,
}

/// Champs pédagogiques communs, jamais transmis au navigateur de l'élève tant
/// que la configuration de l'évaluation n'en autorise pas la divulgation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Teaching {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Choice {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub correct: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Le point n'est obtenu que si l'ensemble exact des réponses est coché.
    #[default]
    Exact,
    /// Une partie des points peut être obtenue.
    Partial,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Scoring {
    #[serde(default)]
    pub mode: Mode,
}

fn one() -> f64 {
    1.0
}

impl Question {
    pub fn id(&self) -> &str {
        match self {
            Question::SingleChoice(q) => &q.id,
            Question::MultipleChoice(q) => &q.id,
            Question::TrueFalse(q) => &q.id,
        }
    }

    pub fn points(&self) -> f64 {
        match self {
            Question::SingleChoice(q) => q.points,
            Question::MultipleChoice(q) => q.points,
            Question::TrueFalse(q) => q.points,
        }
    }

    fn choices(&self) -> &[Choice] {
        match self {
            Question::SingleChoice(q) => &q.choices,
            Question::MultipleChoice(q) => &q.choices,
            Question::TrueFalse(_) => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ValidationError {
    /// Chemin dans le document, par exemple `questions[2].choices[1].id`.
    pub path: String,
    pub message: String,
}

fn err(path: impl Into<String>, message: impl Into<String>) -> ValidationError {
    ValidationError {
        path: path.into(),
        message: message.into(),
    }
}

impl Document {
    pub fn from_yaml(src: &str) -> Result<Self, serde_norway::Error> {
        serde_norway::from_str(src)
    }

    pub fn total_points(&self) -> f64 {
        self.questions.iter().map(Question::points).sum()
    }

    /// Validations métier de la section 9 de la SPEC.
    ///
    /// Renvoie toutes les erreurs d'un coup : un agent qui corrige son YAML a
    /// besoin de la liste complète, pas de la première anomalie rencontrée.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if self.schema != SCHEMA {
            errors.push(err(
                "schema",
                format!("attendu `{SCHEMA}`, trouvé `{}`", self.schema),
            ));
        }
        if self.metadata.title.trim().is_empty() {
            errors.push(err("metadata.title", "titre vide"));
        }
        if self.questions.is_empty() {
            errors.push(err("questions", "aucune question"));
        }

        let mut seen = std::collections::HashSet::new();
        for (i, q) in self.questions.iter().enumerate() {
            let at = format!("questions[{i}]");

            if q.id().trim().is_empty() {
                errors.push(err(format!("{at}.id"), "identifiant vide"));
            } else if !seen.insert(q.id()) {
                errors.push(err(
                    format!("{at}.id"),
                    format!("identifiant de question dupliqué : `{}`", q.id()),
                ));
            }

            if !(q.points() > 0.0) {
                errors.push(err(
                    format!("{at}.points"),
                    "le barème doit être strictement positif",
                ));
            }

            let choices = q.choices();
            let mut seen_choices = std::collections::HashSet::new();
            for (j, c) in choices.iter().enumerate() {
                if !seen_choices.insert(&c.id) {
                    errors.push(err(
                        format!("{at}.choices[{j}].id"),
                        format!("identifiant de réponse dupliqué : `{}`", c.id),
                    ));
                }
                if c.text.trim().is_empty() {
                    errors.push(err(format!("{at}.choices[{j}].text"), "libellé vide"));
                }
            }

            let correct = choices.iter().filter(|c| c.correct).count();
            match q {
                Question::SingleChoice(_) if correct != 1 => errors.push(err(
                    format!("{at}.choices"),
                    format!("single_choice exige exactement une réponse correcte, {correct} trouvée(s)"),
                )),
                Question::MultipleChoice(_) if correct == 0 => errors.push(err(
                    format!("{at}.choices"),
                    "multiple_choice exige au moins une réponse correcte",
                )),
                _ => {}
            }

            if !choices.is_empty() && choices.len() < 2 {
                errors.push(err(format!("{at}.choices"), "au moins deux propositions"));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
schema: qcm/v1
metadata:
  title: "HTTP et architecture Web"
  tags: [http, web]
questions:
  - type: single_choice
    id: http-method-get
    prompt: "Quelle méthode HTTP demande une ressource sans la modifier ?"
    choices:
      - { id: a, text: "POST" }
      - { id: b, text: "GET", correct: true }
    points: 1
    explanation: "GET récupère la représentation d'une ressource."
  - type: multiple_choice
    id: http-codes
    prompt: "Lesquels sont des erreurs côté client ?"
    choices:
      - { id: a, text: "200" }
      - { id: b, text: "404", correct: true }
      - { id: c, text: "403", correct: true }
    scoring: { mode: partial }
    points: 2
  - type: true_false
    id: http-stateless
    prompt: "HTTP est un protocole sans état."
    answer: true
"#;

    fn parse(src: &str) -> Document {
        Document::from_yaml(src).expect("YAML parsable")
    }

    #[test]
    fn document_valide() {
        let doc = parse(VALID);
        assert!(doc.validate().is_ok(), "{:?}", doc.validate());
        assert_eq!(doc.questions.len(), 3);
        assert_eq!(doc.total_points(), 4.0);
        assert!(matches!(doc.questions[1], Question::MultipleChoice(_)));
    }

    #[test]
    fn scoring_lu_et_par_defaut_exact() {
        let doc = parse(VALID);
        let Question::MultipleChoice(q) = &doc.questions[1] else {
            panic!("attendu multiple_choice");
        };
        assert_eq!(q.scoring.mode, Mode::Partial);

        let sans_scoring = parse(&VALID.replace("    scoring: { mode: partial }\n", ""));
        let Question::MultipleChoice(q) = &sans_scoring.questions[1] else {
            panic!("attendu multiple_choice");
        };
        assert_eq!(q.scoring.mode, Mode::Exact);
    }

    #[test]
    fn single_choice_exige_une_seule_reponse_correcte() {
        let src = VALID.replace(r#"- { id: a, text: "POST" }"#, r#"- { id: a, text: "POST", correct: true }"#);
        let errors = parse(&src).validate().unwrap_err();
        assert!(
            errors.iter().any(|e| e.path == "questions[0].choices"),
            "{errors:?}"
        );
    }

    #[test]
    fn identifiants_de_question_uniques() {
        let src = VALID.replace("id: http-codes", "id: http-method-get");
        let errors = parse(&src).validate().unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("dupliqué")),
            "{errors:?}"
        );
    }

    #[test]
    fn bareme_strictement_positif() {
        let src = VALID.replace("points: 2", "points: 0");
        let errors = parse(&src).validate().unwrap_err();
        assert!(
            errors.iter().any(|e| e.path == "questions[1].points"),
            "{errors:?}"
        );
    }

    /// Le « Norway problem » : avec `serde_yaml`, `no` était désérialisé en
    /// booléen et cassait silencieusement un identifiant de réponse.
    #[test]
    fn identifiant_no_reste_une_chaine() {
        let src = r#"
schema: qcm/v1
metadata:
  title: "Test"
questions:
  - type: single_choice
    id: q1
    prompt: "?"
    choices:
      - { id: no, text: "Non" }
      - { id: yes, text: "Oui", correct: true }
"#;
        let doc = parse(src);
        assert!(doc.validate().is_ok(), "{:?}", doc.validate());
        let Question::SingleChoice(q) = &doc.questions[0] else {
            panic!()
        };
        assert_eq!(q.choices[0].id, "no");
    }
}
