use std::collections::{BTreeMap, BTreeSet};

use crate::metadata::paths::normalize_metadata_path_with_explicit_arrays;
use crate::metadata::validation::normalize_check_specs;

use super::super::{ConfigMetadata, FieldMetadata, ValidationCheckSpec};

impl ConfigMetadata {
    pub(super) fn normalize(&mut self) {
        let mut merged = BTreeMap::<(String, BTreeSet<usize>), FieldMetadata>::new();
        for mut field in self.fields.drain(..) {
            let (path, parsed_explicit_array_segments) =
                normalize_metadata_path_with_explicit_arrays(&field.path);
            field.path = path;
            if field.path_explicit_array_segments.is_empty() {
                field.path_explicit_array_segments = parsed_explicit_array_segments;
            }
            field.alias_explicit_array_segments = normalize_alias_explicit_array_segments(
                field.alias_explicit_array_segments,
                field.aliases.iter().map(String::as_str),
            );
            field.aliases = field
                .aliases
                .into_iter()
                .map(|alias| {
                    let (alias, parsed_explicit_array_segments) =
                        normalize_metadata_path_with_explicit_arrays(&alias);
                    if !parsed_explicit_array_segments.is_empty() {
                        field
                            .alias_explicit_array_segments
                            .entry(alias.clone())
                            .or_insert(parsed_explicit_array_segments);
                    }
                    alias
                })
                .filter(|alias| alias != &field.path)
                .collect();
            field.aliases.sort();
            field.aliases.dedup();
            field
                .alias_explicit_array_segments
                .retain(|alias, _| field.aliases.contains(alias));
            let key = field_merge_key(&field);
            match merged.get_mut(&key) {
                Some(existing) => existing.merge_from(field),
                None => {
                    merged.insert(key, field);
                }
            }
        }
        self.fields = merged.into_values().collect();
        let pending_check_specs = self
            .pending_checks
            .drain(..)
            .filter_map(ValidationCheckSpec::from_public);
        self.check_specs =
            normalize_check_specs(self.check_specs.drain(..).chain(pending_check_specs));
        self.checks = self
            .check_specs
            .iter()
            .map(ValidationCheckSpec::to_public)
            .collect();
    }
}

fn field_merge_key(field: &FieldMetadata) -> (String, BTreeSet<usize>) {
    (
        field.path.clone(),
        field.path_explicit_array_segments.clone(),
    )
}

fn normalize_alias_explicit_array_segments<'a>(
    explicit: BTreeMap<String, BTreeSet<usize>>,
    aliases: impl Iterator<Item = &'a str>,
) -> BTreeMap<String, BTreeSet<usize>> {
    let mut normalized = BTreeMap::new();
    for alias in aliases {
        let (normalized_alias, parsed_explicit_array_segments) =
            normalize_metadata_path_with_explicit_arrays(alias);
        if !parsed_explicit_array_segments.is_empty() {
            normalized.insert(normalized_alias.clone(), parsed_explicit_array_segments);
        }
        if let Some(explicit_array_segments) = explicit.get(alias) {
            normalized.insert(normalized_alias, explicit_array_segments.clone());
        }
    }
    normalized
}
