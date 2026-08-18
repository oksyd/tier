use crate::path::{get_value_at_path, redact_value};

use super::{
    ConfigReport, Explanation,
    lookup::{normalize_lookup_path, path_overlaps_secret},
};

impl ConfigReport {
    /// Returns the final configuration value with secret paths redacted.
    #[must_use]
    pub fn redacted_value(&self) -> serde_json::Value {
        redact_value(&self.final_value, "", &self.secret_paths)
    }

    /// Explains how a configuration path was resolved.
    #[must_use]
    pub fn explain(&self, path: &str) -> Option<Explanation> {
        let normalized =
            normalize_lookup_path(path, &self.final_value, &self.alias_overrides, &self.traces)?;
        let redacted = path_overlaps_secret(&normalized, &self.secret_paths);
        let steps = self
            .traces
            .get(&normalized)?
            .iter()
            .cloned()
            .map(|mut step| {
                if redacted {
                    step.value = redact_value(&step.value, &normalized, &self.secret_paths);
                    step.redacted = true;
                }
                step
            })
            .collect();
        let final_value = get_value_at_path(&self.final_value, &normalized)
            .cloned()
            .map(|value| redact_value(&value, &normalized, &self.secret_paths));

        Some(Explanation {
            path: normalized,
            final_value,
            steps,
            redacted,
        })
    }
}
