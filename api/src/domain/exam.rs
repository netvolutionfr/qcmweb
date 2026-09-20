//! Vue d'un sujet telle qu'elle est transmise à un élève en cours d'épreuve.
//!
//! Les types de ce module **ne possèdent pas de champ** `correct`, `explanation`
//! ni de barème détaillé. Ce n'est pas un oubli de sérialisation que l'on
//! pourrait rétablir par mégarde : la fuite est impossible à écrire, puisque
//! l'information n'existe pas dans la structure envoyée (SPEC §12).
//!
//! Ouvrir les outils de développement du navigateur ne révèle donc rien.

use rand::{Rng, SeedableRng};
use serde::Serialize;

use super::qcm::{Document, Question};

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ExamChoice {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ExamQuestion {
    pub id: String,
    /// `single_choice`, `multiple_choice` ou `true_false`.
    pub kind: String,
    pub prompt: String,
    pub points: f64,
    /// Plusieurs réponses peuvent être cochées.
    pub multiple: bool,
    pub choices: Vec<ExamChoice>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Exam {
    pub title: String,
    pub description: Option<String>,
    pub total_points: f64,
    pub questions: Vec<ExamQuestion>,
}

/// Construit la vue élève, mélangée de façon déterministe.
///
/// Le mélange dépend d'une graine conservée sur la tentative, et non d'un
/// tirage à chaque appel : une tentative reprise après une coupure réseau doit
/// retrouver exactement le même ordre, faute de quoi l'élève ne s'y
/// retrouverait plus (SPEC §12).
pub fn build(document: &Document, seed: i64, shuffle_questions: bool, shuffle_choices: bool) -> Exam {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);

    let mut questions: Vec<ExamQuestion> = document
        .questions
        .iter()
        .map(|question| {
            let (kind, multiple, mut choices) = match question {
                Question::SingleChoice(q) => (
                    "single_choice",
                    false,
                    q.choices
                        .iter()
                        .map(|c| ExamChoice {
                            id: c.id.clone(),
                            text: c.text.clone(),
                        })
                        .collect::<Vec<_>>(),
                ),
                Question::MultipleChoice(q) => (
                    "multiple_choice",
                    true,
                    q.choices
                        .iter()
                        .map(|c| ExamChoice {
                            id: c.id.clone(),
                            text: c.text.clone(),
                        })
                        .collect(),
                ),
                // Vrai/Faux garde son ordre naturel : le mélanger n'apporte
                // rien et déroute l'élève.
                Question::TrueFalse(_) => (
                    "true_false",
                    false,
                    vec![
                        ExamChoice {
                            id: "true".into(),
                            text: "Vrai".into(),
                        },
                        ExamChoice {
                            id: "false".into(),
                            text: "Faux".into(),
                        },
                    ],
                ),
            };

            if shuffle_choices && !matches!(question, Question::TrueFalse(_)) {
                shuffle(&mut choices, &mut rng);
            }

            ExamQuestion {
                id: question.id().to_string(),
                kind: kind.to_string(),
                prompt: question.prompt().to_string(),
                points: question.points(),
                multiple,
                choices,
            }
        })
        .collect();

    if shuffle_questions {
        shuffle(&mut questions, &mut rng);
    }

    Exam {
        title: document.metadata.title.clone(),
        description: document.metadata.description.clone(),
        total_points: document.total_points(),
        questions,
    }
}

/// Fisher-Yates, écrit à la main plutôt qu'emprunté à `rand::seq` : le mélange
/// doit rester identique d'une version de dépendance à l'autre, sans quoi une
/// tentative reprise après une mise à jour changerait d'ordre.
fn shuffle<T>(items: &mut [T], rng: &mut impl Rng) {
    for i in (1..items.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"
schema: qcm/v1
metadata: { title: "Test", description: "Consignes" }
questions:
  - type: single_choice
    id: q1
    prompt: "Question un"
    choices:
      - { id: a, text: "A" }
      - { id: b, text: "B", correct: true }
      - { id: c, text: "C" }
      - { id: d, text: "D" }
    points: 1
    explanation: "SECRET"
  - type: multiple_choice
    id: q2
    prompt: "Question deux"
    choices:
      - { id: a, text: "A", correct: true }
      - { id: b, text: "B" }
      - { id: c, text: "C" }
    points: 2
    explanation: "SECRET"
  - type: true_false
    id: q3
    prompt: "Question trois"
    answer: true
    points: 1
    explanation: "SECRET"
"#;

    fn doc() -> Document {
        Document::from_yaml(DOC).unwrap()
    }

    #[test]
    fn aucune_reponse_ni_explication_dans_la_charge_utile() {
        let exam = build(&doc(), 42, true, true);
        let json = serde_json::to_string(&exam).unwrap();

        assert!(!json.contains("correct"), "une bonne réponse a fuité : {json}");
        assert!(!json.contains("SECRET"), "une explication a fuité");
        assert!(!json.contains("explanation"));
        assert!(!json.contains("answer"), "la réponse d'un vrai/faux a fuité");
    }

    #[test]
    fn meme_graine_meme_ordre() {
        let a = build(&doc(), 7, true, true);
        let b = build(&doc(), 7, true, true);
        let order = |e: &Exam| {
            e.questions
                .iter()
                .map(|q| format!("{}:{}", q.id, q.choices.iter().map(|c| c.id.clone()).collect::<Vec<_>>().join("")))
                .collect::<Vec<_>>()
        };
        assert_eq!(order(&a), order(&b), "une reprise doit retrouver le même ordre");
    }

    #[test]
    fn graines_differentes_ordres_differents() {
        let orders: std::collections::HashSet<String> = (0..20)
            .map(|seed| {
                build(&doc(), seed, true, true)
                    .questions
                    .iter()
                    .map(|q| q.id.clone())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect();
        assert!(orders.len() > 1, "le mélange ne mélange rien");
    }

    #[test]
    fn le_melange_ne_perd_ni_ne_duplique_rien() {
        for seed in 0..50 {
            let exam = build(&doc(), seed, true, true);
            assert_eq!(exam.questions.len(), 3);

            let ids: std::collections::HashSet<&str> =
                exam.questions.iter().map(|q| q.id.as_str()).collect();
            assert_eq!(ids.len(), 3, "question perdue ou dupliquée, graine {seed}");

            let q1 = exam.questions.iter().find(|q| q.id == "q1").unwrap();
            let choices: std::collections::HashSet<&str> =
                q1.choices.iter().map(|c| c.id.as_str()).collect();
            assert_eq!(choices.len(), 4, "proposition perdue, graine {seed}");
        }
    }

    #[test]
    fn sans_melange_lordre_du_document_est_conserve() {
        let exam = build(&doc(), 99, false, false);
        assert_eq!(
            exam.questions.iter().map(|q| q.id.as_str()).collect::<Vec<_>>(),
            ["q1", "q2", "q3"]
        );
        assert_eq!(
            exam.questions[0].choices.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c", "d"]
        );
    }

    #[test]
    fn vrai_faux_garde_son_ordre_naturel() {
        for seed in 0..20 {
            let exam = build(&doc(), seed, true, true);
            let q3 = exam.questions.iter().find(|q| q.id == "q3").unwrap();
            assert_eq!(
                q3.choices.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
                ["true", "false"]
            );
        }
    }

    #[test]
    fn nature_de_la_question_transmise() {
        let exam = build(&doc(), 1, false, false);
        assert_eq!(exam.questions[0].kind, "single_choice");
        assert!(!exam.questions[0].multiple);
        assert!(exam.questions[1].multiple);
        assert_eq!(exam.total_points, 4.0);
    }
}
