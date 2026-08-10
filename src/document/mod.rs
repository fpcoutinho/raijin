mod labels;
mod sections;
pub mod template;

pub use labels::finding_category_label;
pub use sections::{appendix_findings, sections};

use crate::domain::{Circuit, ExternalInfluences, InspectionPlanning, QualitativeAssessment, QuantitativeAssessment};

/// Dados de um laudo prontos para virar texto — determinístico ou por IA,
/// ver src/llm/prompt.rs. Deliberadamente sem `location_code` nem
/// `responsible_parties`: identificação de edificação e de pessoa real não
/// entra em nenhum texto gerado, mesma razão do bucket de imagens ser
/// privado (ver CLAUDE.md).
pub struct ReportInput {
    pub inspection_planning: Option<InspectionPlanning>,
    pub external_influences: Option<ExternalInfluences>,
    pub qualitative_assessment: Option<QualitativeAssessment>,
    pub quantitative_assessment: Option<QuantitativeAssessment>,
    pub circuits: Vec<Circuit>,
    pub required_spare_circuits: Option<u32>,
    pub findings: Vec<Finding>,
}

/// `report_section: None` é achado geral — cai no apêndice, não em seção
/// nenhuma (ver docs/report-template.md §"Consequência" e migrations
/// 0001_initial.sql). Eixo independente de `category`.
pub struct Finding {
    pub category: String,
    pub description: Option<String>,
    pub report_section: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionState {
    Filled,
    NotAssessed,
}

/// Uma seção do laudo já resolvida em rótulo pt-BR — o esqueleto que o
/// caminho determinístico (`template::render`) e o prompt da IA
/// (`llm::prompt::build_request`) compartilham. Ligar o toggle de IA troca
/// a redação, não a estrutura: os dois consomem o mesmo `Vec<Section>`.
pub struct Section {
    pub key: &'static str,
    pub title: &'static str,
    pub entries: Vec<(String, String)>,
    pub state: SectionState,
    pub findings: Vec<Finding>,
}
