mod labels;
mod sections;
pub mod template;

pub use labels::finding_category_label;
pub use sections::{appendix_findings, sections};

use uuid::Uuid;

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
#[derive(Clone)]
pub struct Finding {
    /// Sai no texto como marcador `![rótulo](image:<id>)`, e não como URL: a
    /// URL de leitura é assinada e de validade curta, e o `itui` persiste o
    /// documento editado — URL embutida apodreceria dentro do laudo salvo.
    pub image_id: Uuid,
    pub category: String,
    pub description: Option<String>,
    pub report_section: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionState {
    Filled,
    NotAssessed,
}

/// Colunas verbatim de docs/report-template.md — é o que permite ao laudo
/// gerado ter a mesma grade do formulário de origem em vez de uma lista
/// achatada. Cada célula é uma coluna de verdade: cláusula, classificação e
/// observação não são concatenadas no rótulo, senão o `itui` teria que
/// separá-las de novo por regex na hora de montar a tabela do TipTap.
pub struct Table {
    /// Sub-tabela nomeada dentro de uma seção ("Parte I — Medições"), como na
    /// Tabela 10 do modelo. `None` quando a seção tem uma tabela só.
    pub caption: Option<&'static str>,
    /// Primeira linha do cabeçalho quando o modelo agrupa colunas — cada par
    /// é rótulo e quantas colunas ele abrange, somando o total de `headers`.
    /// Vazio na maioria das tabelas; quando existe, o renderizador precisa de
    /// `colspan`, o que Markdown GFM não tem (ver `template::render`).
    pub header_groups: Vec<(&'static str, usize)>,
    pub headers: Vec<&'static str>,
    pub rows: Vec<Vec<String>>,
}

/// Uma seção do laudo já resolvida em rótulo pt-BR — o esqueleto que o
/// caminho determinístico (`template::render`) e o prompt da IA
/// (`llm::prompt::build_request`) compartilham. Ligar o toggle de IA troca
/// a redação, não a estrutura: os dois consomem o mesmo `Vec<Section>`.
pub struct Section {
    pub key: &'static str,
    pub title: &'static str,
    pub tables: Vec<Table>,
    pub state: SectionState,
    pub findings: Vec<Finding>,
}
