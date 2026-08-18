use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::loader::SourceTrace;

use super::{AppliedMigration, ConfigReport, ConfigWarning, ResolutionStep};

impl ConfigReport {
    pub(crate) fn new(
        final_value: Value,
        secret_paths: BTreeSet<String>,
        alias_overrides: BTreeMap<String, String>,
    ) -> Self {
        Self {
            final_value,
            secret_paths,
            alias_overrides,
            traces: BTreeMap::new(),
            applied_sources: Vec::new(),
            validations: Vec::new(),
            warnings: Vec::new(),
            migrations: Vec::new(),
        }
    }

    pub(crate) fn record_source(&mut self, source: SourceTrace) {
        self.applied_sources.push(source);
    }

    pub(crate) fn record_step(&mut self, path: String, step: ResolutionStep) {
        self.traces.entry(path).or_default().push(step);
    }

    pub(crate) fn replace_final_value(&mut self, final_value: Value) {
        self.final_value = final_value;
    }

    pub(crate) fn replace_runtime_metadata(
        &mut self,
        secret_paths: BTreeSet<String>,
        alias_overrides: BTreeMap<String, String>,
    ) {
        self.secret_paths = secret_paths;
        self.alias_overrides = alias_overrides;
    }

    pub(crate) fn record_validation(&mut self, name: String) {
        self.validations.push(name);
    }

    pub(crate) fn record_warning(&mut self, warning: ConfigWarning) {
        self.warnings.push(warning);
    }

    pub(crate) fn record_migration(&mut self, migration: AppliedMigration) {
        self.migrations.push(migration);
    }
}
