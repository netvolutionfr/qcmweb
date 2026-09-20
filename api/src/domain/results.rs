//! Agrégation des résultats d'une évaluation (SPEC §15).
//!
//! Pur : des participants, des tentatives, une table. Aucune base, aucune
//! horloge. Le serveur ne connaissant que des jetons, rien ici n'est nominatif —
//! la jointure avec les noms se fait dans le navigateur de l'enseignant.

use serde::{Deserialize, Serialize};

use super::grading::Grading;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Standing {
    /// A remis au moins une copie.
    Submitted,
    /// A commencé, n'a pas remis.
    InProgress,
    /// N'a jamais commencé.
    Absent,
}

/// Une tentative, telle que la lit l'agrégation.
#[derive(Debug, Clone)]
pub struct AttemptFacts {
    pub participant_id: uuid::Uuid,
    pub started_at: time::OffsetDateTime,
    pub submitted_at: Option<time::OffsetDateTime>,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub grade: Option<f64>,
    pub breakdown: Option<Grading>,
}

#[derive(Debug, Clone)]
pub struct ParticipantFacts {
    pub id: uuid::Uuid,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResultRow {
    pub participant_id: uuid::Uuid,
    pub token: String,
    pub standing: Standing,
    /// Nombre de copies remises.
    pub attempts: i32,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub percentage: Option<f64>,
    pub grade: Option<f64>,
    /// Durée de la copie retenue, en secondes.
    pub duration_seconds: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct QuestionStat {
    pub question_id: String,
    pub max_points: f64,
    /// Part des copies ayant obtenu la totalité des points.
    pub success_rate: f64,
    /// Moyenne des points obtenus, en pourcentage du barème.
    ///
    /// Distincte du taux de réussite : en notation partielle, une question à
    /// 85 % de moyenne et 20 % de réussite complète ne raconte pas du tout la
    /// même histoire qu'une question à 85 % des deux.
    pub average_rate: f64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResultsTable {
    pub rows: Vec<ResultRow>,
    pub questions: Vec<QuestionStat>,
    pub submitted: i32,
    pub in_progress: i32,
    pub absent: i32,
    /// Moyenne des notes des copies remises.
    pub average_grade: Option<f64>,
}

/// Construit la table.
///
/// Lorsque plusieurs tentatives sont autorisées, **la meilleure copie compte**,
/// départagée par la plus précoce à score égal. C'est la lecture habituelle
/// d'une évaluation à plusieurs essais : on mesure ce que l'élève a fini par
/// maîtriser, pas sa première approche.
pub fn build(participants: &[ParticipantFacts], attempts: &[AttemptFacts]) -> ResultsTable {
    let mut rows = Vec::with_capacity(participants.len());

    for participant in participants {
        let mine: Vec<&AttemptFacts> = attempts
            .iter()
            .filter(|a| a.participant_id == participant.id)
            .collect();

        let submitted: Vec<&&AttemptFacts> =
            mine.iter().filter(|a| a.submitted_at.is_some()).collect();

        let best = submitted
            .iter()
            .copied()
            .max_by(|a, b| {
                let score = a
                    .score
                    .unwrap_or(0.0)
                    .partial_cmp(&b.score.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal);
                // À score égal, la copie remise en premier l'emporte.
                score.then_with(|| b.submitted_at.cmp(&a.submitted_at))
            })
            .copied();

        let standing = match (best.is_some(), mine.is_empty()) {
            (true, _) => Standing::Submitted,
            (false, false) => Standing::InProgress,
            (false, true) => Standing::Absent,
        };

        rows.push(ResultRow {
            participant_id: participant.id,
            token: participant.token.clone(),
            standing,
            attempts: submitted.len() as i32,
            score: best.and_then(|a| a.score),
            max_score: best.and_then(|a| a.max_score),
            percentage: best.and_then(|a| a.breakdown.as_ref().map(|b| b.percentage)),
            grade: best.and_then(|a| a.grade),
            duration_seconds: best.and_then(|a| {
                a.submitted_at
                    .map(|end| (end - a.started_at).whole_seconds())
            }),
            submitted_at: best.and_then(|a| a.submitted_at),
        });
    }

    rows.sort_by(|a, b| a.token.cmp(&b.token));

    let counted = |s: Standing| rows.iter().filter(|r| r.standing == s).count() as i32;
    let grades: Vec<f64> = rows.iter().filter_map(|r| r.grade).collect();

    ResultsTable {
        questions: question_stats(participants, attempts),
        submitted: counted(Standing::Submitted),
        in_progress: counted(Standing::InProgress),
        absent: counted(Standing::Absent),
        average_grade: (!grades.is_empty())
            .then(|| round(grades.iter().sum::<f64>() / grades.len() as f64)),
        rows,
    }
}

/// Statistiques par question, calculées sur les copies retenues.
fn question_stats(
    participants: &[ParticipantFacts],
    attempts: &[AttemptFacts],
) -> Vec<QuestionStat> {
    // On reconstruit la sélection « meilleure copie » pour que les statistiques
    // portent exactement sur les copies affichées dans le tableau.
    let kept: Vec<&Grading> = participants
        .iter()
        .filter_map(|p| {
            attempts
                .iter()
                .filter(|a| a.participant_id == p.id && a.submitted_at.is_some())
                .max_by(|a, b| {
                    a.score
                        .unwrap_or(0.0)
                        .partial_cmp(&b.score.unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| b.submitted_at.cmp(&a.submitted_at))
                })
                .and_then(|a| a.breakdown.as_ref())
        })
        .collect();

    let Some(first) = kept.first() else {
        return vec![];
    };

    first
        .per_question
        .iter()
        .map(|reference| {
            let scores: Vec<&super::grading::QuestionScore> = kept
                .iter()
                .filter_map(|g| {
                    g.per_question
                        .iter()
                        .find(|q| q.question_id == reference.question_id)
                })
                .collect();

            let total = scores.len().max(1) as f64;
            let full = scores.iter().filter(|q| q.correct).count() as f64;
            let earned: f64 = scores.iter().map(|q| q.points).sum();
            let available = reference.max_points * total;

            QuestionStat {
                question_id: reference.question_id.clone(),
                max_points: reference.max_points,
                success_rate: round(full / total * 100.0),
                average_rate: if available > 0.0 {
                    round(earned / available * 100.0)
                } else {
                    0.0
                },
            }
        })
        .collect()
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Export CSV pseudonymisé.
///
/// Séparateur `;` et **BOM UTF-8** : sans lui, un tableur francophone ouvre le
/// fichier en Latin-1 et affiche « Ã© » à la place des accents. Le détail
/// n'est pas cosmétique, c'est la différence entre un export exploitable et un
/// export que l'enseignant doit reprendre à la main.
pub fn to_csv(table: &ResultsTable) -> String {
    let mut out = String::from("\u{FEFF}jeton;etat;copies;score;bareme;pourcentage;note;duree_secondes;remise_le\n");

    for row in &table.rows {
        let field = |value: Option<f64>| value.map(|v| v.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{}\n",
            escape(&row.token),
            match row.standing {
                Standing::Submitted => "remis",
                Standing::InProgress => "en cours",
                Standing::Absent => "absent",
            },
            row.attempts,
            field(row.score),
            field(row.max_score),
            field(row.percentage),
            field(row.grade),
            row.duration_seconds.map(|d| d.to_string()).unwrap_or_default(),
            row.submitted_at
                .and_then(|t| t.format(&time::format_description::well_known::Rfc3339).ok())
                .unwrap_or_default(),
        ));
    }
    out
}

fn escape(value: &str) -> String {
    if value.contains([';', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::grading::QuestionScore;
    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    fn at(minutes: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::minutes(minutes)
    }

    fn grading(points: &[(&str, f64, f64)]) -> Grading {
        let per_question: Vec<QuestionScore> = points
            .iter()
            .map(|(id, p, max)| QuestionScore {
                question_id: id.to_string(),
                points: *p,
                max_points: *max,
                correct: (p - max).abs() < f64::EPSILON,
            })
            .collect();
        let score: f64 = per_question.iter().map(|q| q.points).sum();
        let max_score: f64 = per_question.iter().map(|q| q.max_points).sum();
        Grading {
            score,
            max_score,
            percentage: score / max_score * 100.0,
            per_question,
        }
    }

    fn attempt(who: Uuid, start: i64, end: Option<i64>, points: &[(&str, f64, f64)]) -> AttemptFacts {
        let g = grading(points);
        AttemptFacts {
            participant_id: who,
            started_at: at(start),
            submitted_at: end.map(at),
            score: end.map(|_| g.score),
            max_score: end.map(|_| g.max_score),
            grade: end.map(|_| g.score / g.max_score * 20.0),
            breakdown: end.map(|_| g),
        }
    }

    fn people(n: usize) -> Vec<ParticipantFacts> {
        (0..n)
            .map(|i| ParticipantFacts {
                id: Uuid::from_u128(i as u128 + 1),
                token: format!("JETON-{i:02}"),
            })
            .collect()
    }

    #[test]
    fn trois_etats_distingues() {
        let p = people(3);
        let attempts = vec![
            attempt(p[0].id, 0, Some(12), &[("q1", 1.0, 1.0)]),
            attempt(p[1].id, 0, None, &[]),
        ];
        let table = build(&p, &attempts);

        assert_eq!(table.rows[0].standing, Standing::Submitted);
        assert_eq!(table.rows[1].standing, Standing::InProgress);
        assert_eq!(table.rows[2].standing, Standing::Absent);
        assert_eq!((table.submitted, table.in_progress, table.absent), (1, 1, 1));
    }

    #[test]
    fn labsent_figure_dans_le_tableau() {
        let table = build(&people(2), &[]);
        assert_eq!(table.rows.len(), 2, "un absent doit apparaître, pas disparaître");
        assert!(table.rows.iter().all(|r| r.score.is_none()));
    }

    #[test]
    fn la_meilleure_copie_compte() {
        let p = people(1);
        let attempts = vec![
            attempt(p[0].id, 0, Some(10), &[("q1", 1.0, 2.0)]),
            attempt(p[0].id, 20, Some(30), &[("q1", 2.0, 2.0)]),
        ];
        let table = build(&p, &attempts);
        assert_eq!(table.rows[0].score, Some(2.0));
        assert_eq!(table.rows[0].attempts, 2);
    }

    #[test]
    fn a_score_egal_la_plus_precoce() {
        let p = people(1);
        let attempts = vec![
            attempt(p[0].id, 0, Some(10), &[("q1", 1.0, 2.0)]),
            attempt(p[0].id, 20, Some(30), &[("q1", 1.0, 2.0)]),
        ];
        let table = build(&p, &attempts);
        assert_eq!(table.rows[0].duration_seconds, Some(600));
    }

    #[test]
    fn duree_de_la_copie_retenue() {
        let p = people(1);
        let table = build(&p, &[attempt(p[0].id, 5, Some(19), &[("q1", 1.0, 1.0)])]);
        assert_eq!(table.rows[0].duration_seconds, Some(14 * 60));
    }

    #[test]
    fn statistiques_par_question() {
        let p = people(4);
        // q1 : 3 réussites sur 4. q2 : notation partielle, personne au maximum.
        let attempts = vec![
            attempt(p[0].id, 0, Some(1), &[("q1", 1.0, 1.0), ("q2", 1.0, 2.0)]),
            attempt(p[1].id, 0, Some(1), &[("q1", 1.0, 1.0), ("q2", 1.0, 2.0)]),
            attempt(p[2].id, 0, Some(1), &[("q1", 1.0, 1.0), ("q2", 1.0, 2.0)]),
            attempt(p[3].id, 0, Some(1), &[("q1", 0.0, 1.0), ("q2", 1.0, 2.0)]),
        ];
        let table = build(&p, &attempts);

        let q1 = &table.questions[0];
        assert_eq!(q1.success_rate, 75.0);
        assert_eq!(q1.average_rate, 75.0);

        let q2 = &table.questions[1];
        assert_eq!(q2.success_rate, 0.0, "personne n'a la totalité des points");
        assert_eq!(q2.average_rate, 50.0, "mais la moitié des points en moyenne");
    }

    #[test]
    fn statistiques_ignorent_les_copies_non_remises() {
        let p = people(2);
        let attempts = vec![
            attempt(p[0].id, 0, Some(1), &[("q1", 1.0, 1.0)]),
            attempt(p[1].id, 0, None, &[]),
        ];
        let table = build(&p, &attempts);
        assert_eq!(table.questions[0].success_rate, 100.0);
    }

    #[test]
    fn moyenne_sur_les_copies_remises() {
        let p = people(3);
        let attempts = vec![
            attempt(p[0].id, 0, Some(1), &[("q1", 2.0, 2.0)]), // 20
            attempt(p[1].id, 0, Some(1), &[("q1", 1.0, 2.0)]), // 10
        ];
        let table = build(&p, &attempts);
        assert_eq!(table.average_grade, Some(15.0), "l'absent ne tire pas la moyenne");
    }

    #[test]
    fn csv_avec_bom_et_echappement() {
        let p = vec![ParticipantFacts {
            id: Uuid::from_u128(1),
            token: "A;B".into(),
        }];
        let csv = to_csv(&build(&p, &[]));
        assert!(csv.starts_with('\u{FEFF}'), "BOM absent : accents cassés dans un tableur");
        assert!(csv.contains("\"A;B\""), "séparateur non échappé");
        assert!(csv.contains(";absent;"));
    }

    #[test]
    fn tableau_vide_sans_participant() {
        let table = build(&[], &[]);
        assert!(table.rows.is_empty());
        assert!(table.questions.is_empty());
        assert_eq!(table.average_grade, None);
    }
}
