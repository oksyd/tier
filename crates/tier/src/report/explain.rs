use crate::path::{get_value_at_path, redact_value};

use super::{
    ConfigReport, Explanation,
    lookup::{normalize_lookup_path, path_overlaps_secret},
};

impl ConfigReport {
    /// Returns the final configuration value with secret paths redacted.
    #[must_use]
    pub fn redacted_value(&self) -> serde_json::Value {
        self.redacted_final.clone()
    }

    /// Explains how a configuration path was resolved.
    #[must_use]
    pub fn explain(&self, path: &str) -> Option<Explanation> {
        let normalized = normalize_lookup_path(
            path,
            &self.redacted_final,
            &self.alias_overrides,
            &self.traces,
        )?;
        self.explain_recorded_path(&normalized)
    }

    pub(super) fn explain_recorded_path(&self, normalized: &str) -> Option<Explanation> {
        let redacted = path_overlaps_secret(normalized, &self.secret_paths);
        let steps = self
            .traces
            .get(normalized)?
            .iter()
            .cloned()
            .map(|mut step| {
                if redacted {
                    step.value = redact_value(&step.value, normalized, &self.secret_paths);
                    step.redacted = true;
                }
                step
            })
            .collect();
        let final_value = normalize_lookup_path(
            normalized,
            &self.redacted_final,
            &self.alias_overrides,
            &self.traces,
        )
        .filter(|path| path == normalized)
        .and_then(|path| get_value_at_path(&self.redacted_final, &path))
        .cloned();

        Some(Explanation {
            path: normalized.to_owned(),
            final_value,
            steps,
            redacted,
        })
    }
}
