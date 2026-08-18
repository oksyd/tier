use std::collections::BTreeMap;

use serde_json::Value;

use crate::loader::SourceTrace;

use super::{
    AppliedMigration, ConfigReport, ConfigWarning, ReportSummary, ResolutionStep,
    lookup::normalize_lookup_path,
};

impl ConfigReport {
    /// Returns aggregate counts for machine-readable report consumers.
    #[must_use]
    pub fn summary(&self) -> ReportSummary {
        ReportSummary {
            source_count: self.applied_sources.len(),
            validation_count: self.validations.len(),
            warning_count: self.warnings.len(),
            trace_count: self.traces.len(),
            secret_path_count: self.secret_paths.len(),
            migration_count: self.migrations.len(),
        }
    }

    /// Returns the final merged configuration value before redaction.
    #[must_use]
    pub fn final_value(&self) -> &Value {
        &self.final_value
    }

    /// Returns sources that were applied in order.
    #[must_use]
    pub fn applied_sources(&self) -> &[SourceTrace] {
        &self.applied_sources
    }

    /// Returns successfully executed validator names.
    #[must_use]
    pub fn validations(&self) -> &[String] {
        &self.validations
    }

    /// Returns non-fatal warnings recorded during loading.
    #[must_use]
    pub fn warnings(&self) -> &[ConfigWarning] {
        &self.warnings
    }

    /// Returns applied migration steps recorded during loading.
    #[must_use]
    pub fn migrations(&self) -> &[AppliedMigration] {
        &self.migrations
    }

    /// Returns `true` when the report contains warnings.
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Returns all recorded path traces keyed by normalized path.
    #[must_use]
    pub fn traces(&self) -> &BTreeMap<String, Vec<ResolutionStep>> {
        &self.traces
    }

    pub(crate) fn latest_source_for(&self, path: &str) -> Option<SourceTrace> {
        let path =
            normalize_lookup_path(path, &self.final_value, &self.alias_overrides, &self.traces)?;
        self.traces
            .get(&path)
            .and_then(|steps| steps.last())
            .map(|step| step.source.clone())
    }
}
