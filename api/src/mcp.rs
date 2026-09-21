//! Façade MCP.
//!
//! Ce n'est **pas** un second service : les outils appellent les mêmes
//! fonctions que la façade REST, derrière la même authentification et les mêmes
//! autorisations (ADR-0003). Elle n'ouvre aucun chemin d'accès qui n'existe pas
//! déjà.
//!
//! Le partage des pouvoirs est porté par le scope de la clé, non par la
//! description des outils : on ne trouvera donc ici **ni validation d'un
//! sujet, ni ouverture d'évaluation, ni rien qui touche aux participants ou
//! aux jetons**. Un agent écrit, l'humain publie.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    Implementation, InitializeResult, ListResourcesResult, PaginatedRequestParams, ProtocolVersion,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
    ResourceContents, ServerCapabilities,
    ServerConfig,
};
use rmcp::schemars;
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::Author;
use crate::domain::assessment::{Mode, Release};
use crate::error::AppError;
use crate::routes::{assessments, results, subjects};
use crate::state::AppState;

pub const SCHEMA_URI: &str = "qcm://schema/v1";

#[derive(Clone)]
pub struct QcmTools {
    state: AppState,
}

impl QcmTools {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

// ---------------------------------------------------------------------------
// Paramètres
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DocumentParam {
    /// Document `qcm/v1` complet, en YAML ou en JSON.
    pub document: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SubjectParam {
    /// Identifiant du sujet.
    pub subject_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssessmentParam {
    /// Identifiant de l'évaluation.
    pub assessment_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NewAssessmentParam {
    pub subject_id: String,
    /// Numéro de version. Elle doit avoir été validée par l'enseignant.
    pub version: i32,
    pub group_id: String,
    /// Nom affiché, par exemple « Contrôle HTTP du 12 mars ».
    pub name: String,
    /// `FORMATIVE` (sans enjeu de note) ou `SUMMATIVE`.
    pub mode: String,
    /// Durée en minutes. Omise, l'épreuve n'est pas limitée dans le temps.
    pub duration_minutes: Option<i32>,
    /// Nombre de tentatives autorisées. Par défaut 1.
    pub max_attempts: Option<i32>,
    /// `IMMEDIATE`, `AFTER_CLOSE` ou `NEVER`. Par défaut `AFTER_CLOSE`.
    pub score_release: Option<String>,
    /// Idem pour la correction détaillée.
    pub correction_release: Option<String>,
}

// ---------------------------------------------------------------------------
// Outils
// ---------------------------------------------------------------------------

#[tool_router]
impl QcmTools {
    /// Valide un document qcm/v1 sans rien écrire. Renvoie toutes les erreurs
    /// d'un coup, syntaxe et règles métier confondues, afin de permettre une
    /// correction en une seule passe.
    #[tool(name = "qcm_validate_subject")]
    async fn validate_subject(
        &self,
        Parameters(param): Parameters<DocumentParam>,
    ) -> Result<String, ErrorData> {
        Ok(match subjects::parse(&param.document) {
            Ok(document) => json(&serde_json::json!({
                "valid": true,
                "title": document.metadata.title,
                "question_count": document.questions.len(),
                "total_points": document.total_points(),
            })),
            Err(errors) => json(&serde_json::json!({ "valid": false, "errors": errors })),
        })
    }

    /// Dépose un sujet dans la banque. Le résultat est toujours un brouillon :
    /// sa publication relève de l'enseignant, qui le relit dans son navigateur
    /// à l'adresse renvoyée.
    #[tool(name = "qcm_publish_draft")]
    async fn publish_draft(
        &self,
        Parameters(param): Parameters<DocumentParam>,
    ) -> Result<String, ErrorData> {
        let deposited = subjects::deposit_document(&self.state, Author::Agent, &param.document)
            .await
            .map_err(fault)?;
        Ok(json(&deposited))
    }

    /// Liste les sujets de la banque, avec le numéro et l'état de leur dernière
    /// version.
    #[tool(name = "qcm_list_subjects")]
    async fn list_subjects(&self) -> Result<String, ErrorData> {
        Ok(json(&subjects::list_subjects(&self.state).await.map_err(fault)?))
    }

    /// Détaille un sujet et toutes ses versions.
    #[tool(name = "qcm_get_subject")]
    async fn get_subject(
        &self,
        Parameters(param): Parameters<SubjectParam>,
    ) -> Result<String, ErrorData> {
        let id = uuid(&param.subject_id)?;
        Ok(json(&subjects::subject_detail(&self.state, id).await.map_err(fault)?))
    }

    /// Crée une évaluation sur une version validée et renvoie son code d'accès.
    /// L'évaluation naît fermée : son ouverture aux élèves relève de
    /// l'enseignant.
    #[tool(name = "qcm_create_assessment")]
    async fn create_assessment(
        &self,
        Parameters(param): Parameters<NewAssessmentParam>,
    ) -> Result<String, ErrorData> {
        let body = assessments::NewAssessment {
            subject_id: uuid(&param.subject_id)?,
            version: param.version,
            group_id: uuid(&param.group_id)?,
            name: param.name,
            mode: enum_of::<Mode>(&param.mode, "mode")?,
            opens_at: None,
            closes_at: None,
            duration_minutes: param.duration_minutes,
            max_attempts: param.max_attempts.unwrap_or(1),
            shuffle_questions: true,
            shuffle_choices: true,
            score_release: match param.score_release {
                Some(value) => enum_of::<Release>(&value, "score_release")?,
                None => Release::AfterClose,
            },
            correction_release: match param.correction_release {
                Some(value) => enum_of::<Release>(&value, "correction_release")?,
                None => Release::AfterClose,
            },
            max_grade: 20.0,
        };

        let created = assessments::create_assessment(&self.state, Author::Agent, body)
            .await
            .map_err(fault)?;
        Ok(json(&created))
    }

    /// Liste les évaluations, avec leur code, leur état et leur disponibilité.
    #[tool(name = "qcm_list_assessments")]
    async fn list_assessments(&self) -> Result<String, ErrorData> {
        Ok(json(
            &assessments::list_assessments(&self.state).await.map_err(fault)?,
        ))
    }

    /// Résultats d'une évaluation. Les copies y sont désignées par des jetons :
    /// le serveur ne détient aucun nom, et il n'y a donc rien de nominatif à
    /// obtenir ici.
    #[tool(name = "qcm_get_results")]
    async fn get_results(
        &self,
        Parameters(param): Parameters<AssessmentParam>,
    ) -> Result<String, ErrorData> {
        let id = uuid(&param.assessment_id)?;
        Ok(json(&results::collect(&self.state, id).await.map_err(fault)?))
    }

    /// Statistiques par question, entièrement agrégées : taux de réussite
    /// complète et moyenne des points obtenus. Utile pour repérer les notions
    /// insuffisamment maîtrisées par le groupe.
    #[tool(name = "qcm_question_stats")]
    async fn question_stats(
        &self,
        Parameters(param): Parameters<AssessmentParam>,
    ) -> Result<String, ErrorData> {
        let id = uuid(&param.assessment_id)?;
        let table = results::collect(&self.state, id).await.map_err(fault)?;
        Ok(json(&serde_json::json!({
            "questions": table.questions,
            "submitted": table.submitted,
            "average_grade": table.average_grade,
        })))
    }
}

// ---------------------------------------------------------------------------
// Serveur
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for QcmTools {
    fn get_info(&self) -> ServerConfig {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_protocol_version(ProtocolVersion::LATEST)
        .with_server_info(Implementation::new("qcmweb", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "Rédaction de sujets d'évaluation au format qcm/v1.\n\n\
                 Marche à suivre : lire la ressource `qcm://schema/v1`, rédiger le document, \
                 le vérifier avec `qcm_validate_subject` jusqu'à ce qu'il soit valide, puis le \
                 déposer avec `qcm_publish_draft` et transmettre l'adresse de relecture à \
                 l'enseignant.\n\n\
                 Un sujet déposé est un brouillon. Sa validation, l'ouverture d'une évaluation \
                 et tout ce qui touche aux participants relèvent de l'enseignant seul : ces \
                 actions n'existent pas ici, et le demander ne changera rien.",
        )
    }

    async fn list_resources(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut result = ListResourcesResult::default();
        result.resources = vec![
            Resource::new(SCHEMA_URI, "Schéma qcm/v1")
                .with_description(
                    "JSON Schema du format natif : structure du document, types de questions \
                     et règles de notation.",
                )
                .with_mime_type("application/schema+json"),
        ];
        Ok(result)
    }

    async fn read_resource(
        &self,
        param: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        if param.uri != SCHEMA_URI {
            return Err(ErrorData::resource_not_found(
                format!("ressource inconnue : {}", param.uri),
                None,
            ));
        }

        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            crate::routes::qcm_schema(),
            SCHEMA_URI,
        )
        .with_mime_type("application/schema+json")])
        .into())
    }
}

// ---------------------------------------------------------------------------
// Utilitaires
// ---------------------------------------------------------------------------

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

fn uuid(value: &str) -> Result<Uuid, ErrorData> {
    value
        .parse()
        .map_err(|_| ErrorData::invalid_params(format!("identifiant invalide : {value}"), None))
}

fn enum_of<T: serde::de::DeserializeOwned>(value: &str, field: &str) -> Result<T, ErrorData> {
    serde_json::from_value(serde_json::Value::String(value.to_uppercase()))
        .map_err(|_| ErrorData::invalid_params(format!("valeur inattendue pour {field} : {value}"), None))
}

/// Traduit une erreur applicative en erreur MCP.
///
/// Le message du refus est conservé : un agent qui tente de créer une
/// évaluation sur une version non validée doit lire pourquoi, sinon il
/// recommencera à l'identique.
fn fault(error: AppError) -> ErrorData {
    match error {
        AppError::NotFound => ErrorData::invalid_params("ressource introuvable".to_string(), None),
        AppError::Invalid(errors) => ErrorData::invalid_params(
            "document invalide".to_string(),
            Some(serde_json::json!({ "errors": errors })),
        ),
        AppError::Conflict(message) | AppError::BadRequest(message) => {
            ErrorData::invalid_params(message, None)
        }
        other => {
            tracing::error!(error = %other, "erreur pendant un appel MCP");
            ErrorData::internal_error("erreur interne".to_string(), None)
        }
    }
}
