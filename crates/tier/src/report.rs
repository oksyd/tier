use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::loader::SourceTrace;

mod access;
mod explain;
mod lookup;
mod model;
mod output;
mod render;
mod state;

#[cfg(feature = "schema")]
pub use self::model::ExportBundleReport;
pub use self::model::{
    AppliedMigration, AuditReport, ConfigWarning, DeprecatedField, DoctorReport, Explanation,
    ReportSummary, ResolutionStep, TraceAudit,
};

/// Stable version tag for machine-readable doctor and audit reports.
pub const REPORT_FORMAT_VERSION: u32 = 3;

#[cfg(feature = "schema")]
/// Stable version tag for machine-readable export bundles.
pub const EXPORT_BUNDLE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone)]
/// Post-load diagnostics including source traces, warnings, and redacted output helpers.
///
/// `ConfigReport` is returned alongside the final typed configuration and is
/// designed for both humans and tooling:
///
/// - `doctor()` and `doctor_json()` summarize a load at a high level
/// - `explain()` shows how one path was resolved
/// - `audit_report()` and `audit_json()` provide a machine-readable trace for
///   every resolved path
///
/// The report stores only a redacted snapshot. Its `Debug` output and public
/// accessors therefore cannot expose the raw final configuration document.
///
/// # Examples
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use tier::ConfigLoader;
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// impl Default for AppConfig {
///     fn default() -> Self {
///         Self { port: 3000 }
///     }
/// }
///
/// let loaded = ConfigLoader::new(AppConfig::default()).load()?;
/// let doctor = loaded.report().doctor();
/// let explanation = loaded.report().explain("port").expect("port explanation");
///
/// assert!(doctor.contains("Config Doctor"));
/// assert_eq!(explanation.path, "port");
/// # Ok::<(), tier::ConfigError>(())
/// ```
pub struct ConfigReport {
    redacted_final: Value,
    secret_paths: BTreeSet<String>,
    alias_overrides: BTreeMap<String, String>,
    traces: BTreeMap<String, Vec<ResolutionStep>>,
    applied_sources: Vec<SourceTrace>,
    validations: Vec<String>,
    warnings: Vec<ConfigWarning>,
    migrations: Vec<AppliedMigration>,
}
