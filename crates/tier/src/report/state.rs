use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::loader::SourceTrace;
use crate::path::{path_overlaps_pattern, redact_value};

use super::{AppliedMigration, ConfigReport, ConfigWarning, ResolutionStep};

impl ConfigReport {
    pub(crate) fn new(
        final_value: Value,
        secret_paths: BTreeSet<String>,
        alias_overrides: BTreeMap<String, String>,
    ) -> Self {
        let redacted_final = redact_value(&final_value, "", &secret_paths);
        Self {
            redacted_final,
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

    pub(crate) fn secret_paths(&self) -> &BTreeSet<String> {
        &self.secret_paths
    }

    pub(crate) fn extend_secret_paths(&mut self, paths: impl IntoIterator<Item = String>) {
        let mut secrets = self.secret_paths.clone();
        secrets.extend(paths);
        self.replace_runtime_metadata(secrets, self.alias_overrides.clone());
    }

    pub(crate) fn record_migrated_value(
        &mut self,
        from: &str,
        to: &str,
        value: &Value,
        source: SourceTrace,
    ) {
        let value = redact_value(value, from, &self.secret_paths);
        let value = redact_value(&value, to, &self.secret_paths);
        let redacted = self
            .secret_paths
            .iter()
            .any(|secret| path_overlaps_pattern(from, secret) || path_overlaps_pattern(to, secret));
        self.record_step(
            to.to_owned(),
            ResolutionStep {
                source,
                value,
                redacted,
            },
        );
    }

    pub(crate) fn replace_final_value(&mut self, final_value: Value) {
        self.redacted_final = redact_value(&final_value, "", &self.secret_paths);
    }

    pub(crate) fn replace_runtime_metadata(
        &mut self,
        secret_paths: BTreeSet<String>,
        alias_overrides: BTreeMap<String, String>,
    ) {
        self.redacted_final = redact_value(&self.redacted_final, "", &secret_paths);
        for (path, steps) in &mut self.traces {
            let redacted = secret_paths
                .iter()
                .any(|secret| path_overlaps_pattern(path, secret));
            for step in steps {
                step.value = redact_value(&step.value, path, &secret_paths);
                step.redacted |= redacted;
            }
        }
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
