use std::collections::BTreeMap;

use serde_json::Value;

use crate::loader::SourceTrace;

use super::diagnostic::{AppliedMigration, ConfigWarning};
use super::trace::Explanation;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Aggregate counts for a machine-readable configuration report.
pub struct ReportSummary {
    /// Number of applied sources.
    pub source_count: usize,
    /// Number of executed validations.
    pub validation_count: usize,
    /// Number of warnings recorded during loading.
    pub warning_count: usize,
    /// Number of traced configuration paths.
    pub trace_count: usize,
    /// Number of configured secret paths.
    pub secret_path_count: usize,
    /// Number of applied configuration migrations.
    pub migration_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Machine-readable summary of a loaded configuration report.
pub struct DoctorReport {
    /// Stable schema version for external consumers.
    pub format_version: u32,
    /// Aggregate counts for this report.
    pub summary: ReportSummary,
    /// Sources applied in order.
    pub sources: Vec<SourceTrace>,
    /// Validation names executed during loading.
    pub validations: Vec<String>,
    /// Structured warnings recorded during loading.
    pub warnings: Vec<ConfigWarning>,
    /// Applied migration steps recorded during loading.
    pub migrations: Vec<AppliedMigration>,
    /// Final configuration value with redaction applied.
    pub redacted_final: Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Structured audit details for a single resolved path.
pub struct TraceAudit {
    /// Full explanation for the path.
    pub explanation: Explanation,
    /// Most recent source that wrote the path, when known.
    pub last_source: Option<SourceTrace>,
    /// Number of recorded resolution steps for the path.
    pub step_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Machine-readable audit payload including traces for every resolved path.
pub struct AuditReport {
    /// Stable schema version for external consumers.
    pub format_version: u32,
    /// Aggregate counts for this report.
    pub summary: ReportSummary,
    /// Summary doctor payload.
    pub doctor: DoctorReport,
    /// Structured path explanations keyed by normalized path.
    pub traces: BTreeMap<String, TraceAudit>,
}

#[cfg(feature = "schema")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Versioned machine-readable export bundle for downstream integrations.
pub struct ExportBundleReport {
    /// Stable bundle version for external consumers.
    pub format_version: u32,
    /// Operational doctor summary for the loaded configuration.
    pub doctor: DoctorReport,
    /// Full audit payload for the loaded configuration.
    pub audit: AuditReport,
    /// Versioned env docs export.
    pub env_docs: crate::EnvDocsReport,
    /// Versioned plain JSON Schema export.
    pub json_schema: crate::JsonSchemaReport,
    /// Versioned annotated JSON Schema export.
    pub annotated_json_schema: crate::JsonSchemaReport,
    /// Versioned example export.
    pub example: crate::ConfigExampleReport,
}
