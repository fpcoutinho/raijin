//! Tipos de domínio do laudo elétrico. Nomenclatura e estrutura seguem
//! docs/domain-glossary.md.

mod assessment;
mod circuit;
mod image;
mod report;
mod user;

pub use assessment::{
    BinaryAnswer, ExternalInfluences, InspectionPlanning, QualitativeAnswer, QualitativeAssessment,
    QuantitativeAssessment, TernaryAnswer, TestAnswer,
};
pub use circuit::Circuit;
pub use image::{FINDING_CATEGORIES, ImageUploadStatus, ReportImage};
pub use report::{Report, ReportStatus};
pub use user::User;
