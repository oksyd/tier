use std::collections::BTreeMap;

use crate::metadata::paths::render_metadata_path;

use super::super::{ConfigMetadata, MergeStrategy};

impl ConfigMetadata {
    /// Returns explicitly declared field merge strategies keyed by normalized path.
    #[must_use]
    pub fn merge_strategies(&self) -> BTreeMap<String, MergeStrategy> {
        self.fields
            .iter()
            .filter(|field| field.merge_explicit)
            .map(|field| {
                (
                    render_metadata_path(&field.path, &field.path_explicit_array_segments),
                    field.merge,
                )
            })
            .collect()
    }

    /// Resolves the effective merge strategy for a concrete configuration path.
    #[must_use]
    pub fn merge_strategy_for(&self, path: &str) -> Option<MergeStrategy> {
        self.effective_field_for(path).map(|field| field.merge)
    }
}
