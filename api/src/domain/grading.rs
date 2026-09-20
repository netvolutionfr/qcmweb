//! Moteur de correction.
//!
//! Entièrement pur : un document, des réponses, un résultat. Aucune base,
//! aucune horloge, aucun réseau — la correction est ce qui produit des notes,
//! elle doit être reproductible et vérifiable sans rien monter (SPEC §13).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::qcm::{Document, Question};

/// Réponses d'un élève : identifiant de question → identifiants cochés.
pub type Answers = HashMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QuestionScore {
    pub question_id: String,
    pub points: f64,
    pub max_points: f64,
    /// Vrai si l'élève a obtenu la totalité des points.
    pub correct: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Grading {
    pub score: f64,
    pub max_score: f64,
    pub percentage: f64,
    pub per_question: Vec<QuestionScore>,
}

/// Corrige une copie.
///
/// Une question sans réponse vaut zéro, jamais une pénalité : le score minimal
/// d'une question est zéro par défaut (SPEC §7).
pub fn grade(document: &Document, answers: &Answers) -> Grading {
    let empty = Vec::new();
    let per_question: Vec<QuestionScore> = document
        .questions
        .iter()
        .map(|question| {
            let given = answers.get(question.id()).unwrap_or(&empty);
            score_question(question, given)
        })
        .collect();

    let score: f64 = per_question.iter().map(|q| q.points).sum();
    let max_score: f64 = per_question.iter().map(|q| q.max_points).sum();

    Grading {
        score: round(score),
        max_score: round(max_score),
        percentage: if max_score > 0.0 {
            round(score / max_score * 100.0)
        } else {
            0.0
        },
        per_question,
    }
}

fn score_question(question: &Question, given: &[String]) -> QuestionScore {
    let max_points = question.points();
    let selected: HashSet<&str> = given.iter().map(String::as_str).collect();

    let points = match question {
        Question::TrueFalse(q) => {
            let expected = if q.answer { "true" } else { "false" };
            if selected.len() == 1 && selected.contains(expected) {
                max_points
            } else {
                0.0
            }
        }

        Question::SingleChoice(q) => {
            // Plusieurs cases cochées sur une question à réponse unique : la
            // copie ne répond pas à la question posée, elle vaut zéro. Prendre
            // la première cochée reviendrait à récompenser le fait de tout
            // cocher.
            let right = q.choices.iter().find(|c| c.correct).map(|c| c.id.as_str());
            match (selected.len(), right) {
                (1, Some(id)) if selected.contains(id) => max_points,
                _ => 0.0,
            }
        }

        Question::MultipleChoice(q) => {
            let correct: HashSet<&str> = q
                .choices
                .iter()
                .filter(|c| c.correct)
                .map(|c| c.id.as_str())
                .collect();
            let wrong: HashSet<&str> = q
                .choices
                .iter()
                .filter(|c| !c.correct)
                .map(|c| c.id.as_str())
                .collect();

            let hits = selected.intersection(&correct).count() as f64;
            let misses = selected.intersection(&wrong).count() as f64;

            match q.scoring.mode {
                super::qcm::Mode::Exact => {
                    if hits == correct.len() as f64 && misses == 0.0 {
                        max_points
                    } else {
                        0.0
                    }
                }
                // Proportion de bonnes réponses trouvées, diminuée de la
                // proportion de mauvaises cochées. Sans ce second terme, tout
                // cocher garantirait la note maximale.
                super::qcm::Mode::Partial => {
                    let found = hits / correct.len().max(1) as f64;
                    let penalty = if wrong.is_empty() {
                        0.0
                    } else {
                        misses / wrong.len() as f64
                    };
                    max_points * (found - penalty).max(0.0)
                }
            }
        }
    };

    QuestionScore {
        question_id: question.id().to_string(),
        points: round(points),
        max_points,
        correct: (points - max_points).abs() < f64::EPSILON,
    }
}

/// Deux décimales : un barème en points ne se discute pas au millième, et les
/// flottants bruts produiraient des `2.7999999999999998` dans les exports.
fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"
schema: qcm/v1
metadata:
  title: "Test"
questions:
  - type: single_choice
    id: q1
    prompt: "?"
    choices:
      - { id: a, text: "A" }
      - { id: b, text: "B", correct: true }
    points: 2
  - type: multiple_choice
    id: q2
    prompt: "?"
    choices:
      - { id: a, text: "A", correct: true }
      - { id: b, text: "B", correct: true }
      - { id: c, text: "C" }
      - { id: d, text: "D" }
    scoring: { mode: exact }
    points: 4
  - type: true_false
    id: q3
    prompt: "?"
    answer: true
    points: 1
"#;

    fn doc() -> Document {
        Document::from_yaml(DOC).unwrap()
    }

    fn partial_doc() -> Document {
        Document::from_yaml(&DOC.replace("mode: exact", "mode: partial")).unwrap()
    }

    fn answers(pairs: &[(&str, &[&str])]) -> Answers {
        pairs
            .iter()
            .map(|(q, cs)| {
                (
                    q.to_string(),
                    cs.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    fn points_for(g: &Grading, id: &str) -> f64 {
        g.per_question.iter().find(|q| q.question_id == id).unwrap().points
    }

    #[test]
    fn copie_parfaite() {
        let g = grade(&doc(), &answers(&[("q1", &["b"]), ("q2", &["a", "b"]), ("q3", &["true"])]));
        assert_eq!(g.score, 7.0);
        assert_eq!(g.max_score, 7.0);
        assert_eq!(g.percentage, 100.0);
        assert!(g.per_question.iter().all(|q| q.correct));
    }

    #[test]
    fn copie_blanche_vaut_zero_sans_penalite() {
        let g = grade(&doc(), &answers(&[]));
        assert_eq!(g.score, 0.0);
        assert_eq!(g.max_score, 7.0);
        assert_eq!(g.percentage, 0.0);
    }

    #[test]
    fn single_choice_plusieurs_cases_vaut_zero() {
        let g = grade(&doc(), &answers(&[("q1", &["a", "b"])]));
        assert_eq!(
            points_for(&g, "q1"),
            0.0,
            "tout cocher ne doit pas permettre d'obtenir les points"
        );
    }

    #[test]
    fn exact_exige_lensemble_exact() {
        for (given, expected) in [
            (vec!["a", "b"], 4.0),
            (vec!["a"], 0.0),
            (vec!["a", "b", "c"], 0.0),
            (vec!["c", "d"], 0.0),
        ] {
            let g = grade(&doc(), &answers(&[("q2", &given)]));
            assert_eq!(points_for(&g, "q2"), expected, "réponses {given:?}");
        }
    }

    #[test]
    fn partiel_recompense_et_penalise() {
        // 2 bonnes, 2 mauvaises, 4 points.
        for (given, expected) in [
            (vec!["a", "b"], 4.0),       // tout juste
            (vec!["a"], 2.0),            // la moitié des bonnes
            (vec!["a", "c"], 0.0),       // 1/2 trouvée - 1/2 fausse = 0
            (vec!["a", "b", "c"], 2.0),  // 1 - 1/2 = 1/2
            (vec!["c", "d"], 0.0),       // plancher à zéro, jamais négatif
            (vec!["a", "b", "c", "d"], 0.0),
        ] {
            let g = grade(&partial_doc(), &answers(&[("q2", &given)]));
            assert_eq!(points_for(&g, "q2"), expected, "réponses {given:?}");
        }
    }

    #[test]
    fn tout_cocher_ne_paie_jamais() {
        let g = grade(&partial_doc(), &answers(&[("q2", &["a", "b", "c", "d"])]));
        assert_eq!(points_for(&g, "q2"), 0.0);
    }

    #[test]
    fn vrai_faux() {
        assert_eq!(points_for(&grade(&doc(), &answers(&[("q3", &["true"])])), "q3"), 1.0);
        assert_eq!(points_for(&grade(&doc(), &answers(&[("q3", &["false"])])), "q3"), 0.0);
        assert_eq!(
            points_for(&grade(&doc(), &answers(&[("q3", &["true", "false"])])), "q3"),
            0.0
        );
    }

    #[test]
    fn identifiant_inconnu_ignore() {
        let g = grade(&doc(), &answers(&[("q1", &["zzz"]), ("q9", &["a"])]));
        assert_eq!(points_for(&g, "q1"), 0.0);
        assert_eq!(g.per_question.len(), 3, "seules les questions du sujet comptent");
    }

    #[test]
    fn arrondi_au_centieme() {
        // 3 bonnes sur 7 points : 1/3 des points, soit 2.333...
        let src = r#"
schema: qcm/v1
metadata: { title: "T" }
questions:
  - type: multiple_choice
    id: q
    prompt: "?"
    choices:
      - { id: a, text: "A", correct: true }
      - { id: b, text: "B", correct: true }
      - { id: c, text: "C", correct: true }
      - { id: d, text: "D" }
    scoring: { mode: partial }
    points: 7
"#;
        let g = grade(&Document::from_yaml(src).unwrap(), &answers(&[("q", &["a"])]));
        assert_eq!(points_for(&g, "q"), 2.33);
    }

    #[test]
    fn correction_reproductible() {
        let a = answers(&[("q1", &["b"]), ("q2", &["a"])]);
        assert_eq!(grade(&doc(), &a), grade(&doc(), &a));
    }
}
