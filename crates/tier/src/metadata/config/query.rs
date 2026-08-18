use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    ConfigMetadata, EffectiveSourcePolicy, EffectiveValidation, FieldMetadata, MetadataMatchScore,
    ValidationCheckSpec,
};
use crate::metadata::paths::{
    metadata_match_score, render_metadata_path, try_normalize_metadata_path,
    try_normalize_metadata_path_with_explicit_arrays,
};
use crate::path::path_segments;

impl ConfigMetadata {
    /// Returns all merged field metadata entries.
    #[must_use]
    pub fn fields(&self) -> &[FieldMetadata] {
        &self.fields
    }

    /// Returns all normalized cross-field validation checks.
    #[must_use]
    pub fn checks(&self) -> &[super::super::ValidationCheck] {
        &self.checks
    }

    pub(crate) fn check_specs(&self) -> &[ValidationCheckSpec] {
        &self.check_specs
    }

    /// Returns the metadata entry for a normalized configuration path or alias.
    #[must_use]
    pub fn field(&self, path: &str) -> Option<&FieldMetadata> {
        let (normalized, explicit_array_segments) =
            try_normalize_metadata_path_with_explicit_arrays(path).ok()?;
        let mut best = None::<(MetadataMatchScore, &FieldMetadata)>;
        for field in &self.fields {
            if let Some(score) =
                public_field_match_score(&normalized, &explicit_array_segments, field)
            {
                match &mut best {
                    Some((best_score, best_field)) if score > *best_score => {
                        *best_score = score;
                        *best_field = field;
                    }
                    None => best = Some((score, field)),
                    _ => {}
                }
            }
        }

        best.map(|(_, field)| field)
    }

    pub(crate) fn matching_fields_for_path(&self, path: &str) -> Vec<&FieldMetadata> {
        let normalized = match try_normalize_metadata_path(path) {
            Ok(normalized) => normalized,
            Err(_) => return Vec::new(),
        };

        let mut matches = self
            .fields
            .iter()
            .filter_map(|field| {
                let best = std::iter::once(field.path.as_str())
                    .chain(field.aliases.iter().map(String::as_str))
                    .filter_map(|candidate| metadata_match_score(&normalized, candidate))
                    .max();
                best.map(|score| (score, field))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.path.cmp(&right.1.path))
        });
        matches.into_iter().map(|(_, field)| field).collect()
    }

    #[cfg(feature = "schema")]
    pub(crate) fn matching_fields_for_path_with_intent(
        &self,
        path: &str,
        explicit_array_segments: &BTreeSet<usize>,
    ) -> Vec<&FieldMetadata> {
        let (normalized, parsed_explicit_array_segments) =
            match try_normalize_metadata_path_with_explicit_arrays(path) {
                Ok(parsed) => parsed,
                Err(_) => return Vec::new(),
            };
        let mut query_explicit_array_segments = explicit_array_segments.clone();
        query_explicit_array_segments.extend(parsed_explicit_array_segments);

        let mut matches = self
            .fields
            .iter()
            .filter_map(|field| {
                export_field_match_score(&normalized, &query_explicit_array_segments, field)
                    .map(|score| (score, field))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.path.cmp(&right.1.path))
        });
        matches.into_iter().map(|(_, field)| field).collect()
    }

    pub(crate) fn effective_source_policy_for(&self, path: &str) -> Option<EffectiveSourcePolicy> {
        let mut policy = EffectiveSourcePolicy::default();
        let mut has_policy = false;

        for field in self.matching_fields_for_path(path) {
            if field.allowed_sources.is_some() || field.denied_sources.is_some() {
                has_policy = true;
                policy.apply_field(field);
            }
        }

        has_policy.then_some(policy)
    }

    #[cfg(feature = "schema")]
    pub(crate) fn effective_source_policy_for_path_with_intent(
        &self,
        path: &str,
        explicit_array_segments: &BTreeSet<usize>,
    ) -> Option<EffectiveSourcePolicy> {
        let mut policy = EffectiveSourcePolicy::default();
        let mut has_policy = false;

        for field in self.matching_fields_for_path_with_intent(path, explicit_array_segments) {
            if field.allowed_sources.is_some() || field.denied_sources.is_some() {
                has_policy = true;
                policy.apply_field(field);
            }
        }

        has_policy.then_some(policy)
    }

    pub(crate) fn effective_validations_for(&self, path: &str) -> Vec<EffectiveValidation> {
        let Some(field) = self.effective_field_for(path) else {
            return Vec::new();
        };

        field
            .validations
            .iter()
            .cloned()
            .map(|rule| EffectiveValidation {
                field: field.clone(),
                rule,
            })
            .collect()
    }

    pub(crate) fn effective_field_for(&self, path: &str) -> Option<FieldMetadata> {
        let mut matches = self.matching_fields_for_path(path).into_iter();
        let mut effective = matches.next()?.clone();
        for field in matches {
            effective.merge_from(field.clone());
        }
        Some(effective)
    }

    #[cfg(feature = "schema")]
    pub(crate) fn effective_field_for_path_with_intent(
        &self,
        path: &str,
        explicit_array_segments: &BTreeSet<usize>,
    ) -> Option<FieldMetadata> {
        let mut matches = self
            .matching_fields_for_path_with_intent(path, explicit_array_segments)
            .into_iter();
        let mut effective = matches.next()?.clone();
        for field in matches {
            effective.merge_from(field.clone());
        }
        Some(effective)
    }

    /// Returns metadata entries keyed by normalized path.
    #[must_use]
    pub fn fields_by_path(&self) -> BTreeMap<String, FieldMetadata> {
        self.fields
            .iter()
            .cloned()
            .map(|field| {
                (
                    render_metadata_path(&field.path, &field.path_explicit_array_segments),
                    field,
                )
            })
            .collect()
    }

    /// Returns all normalized secret paths.
    #[must_use]
    pub fn secret_paths(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter(|field| field.secret)
            .map(|field| render_metadata_path(&field.path, &field.path_explicit_array_segments))
            .collect()
    }
}

fn public_field_match_score(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    field: &FieldMetadata,
) -> Option<MetadataMatchScore> {
    let mut best = public_candidate_match_score(
        path,
        explicit_array_segments,
        &field.path,
        &field.path_explicit_array_segments,
    );

    for alias in &field.aliases {
        let empty = BTreeSet::new();
        let alias_explicit_array_segments = field
            .alias_explicit_array_segments
            .get(alias)
            .unwrap_or(&empty);
        let Some(score) = public_candidate_match_score(
            path,
            explicit_array_segments,
            alias,
            alias_explicit_array_segments,
        ) else {
            continue;
        };

        match &mut best {
            Some(best_score) if score > *best_score => *best_score = score,
            None => best = Some(score),
            _ => {}
        }
    }

    best
}

fn public_candidate_match_score(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    candidate: &str,
    candidate_explicit_array_segments: &BTreeSet<usize>,
) -> Option<MetadataMatchScore> {
    let score = metadata_match_score(path, candidate)?;
    explicit_array_intent_matches_public_query(
        path,
        explicit_array_segments,
        candidate,
        candidate_explicit_array_segments,
    )
    .then_some(score)
}

fn explicit_array_intent_matches_public_query(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    candidate: &str,
    candidate_explicit_array_segments: &BTreeSet<usize>,
) -> bool {
    let query_segments = path_segments(path);
    let candidate_segments = path_segments(candidate);

    explicit_array_segments
        .symmetric_difference(candidate_explicit_array_segments)
        .all(|index| {
            query_segments.get(*index) == Some(&"*") || candidate_segments.get(*index) == Some(&"*")
        })
}

#[cfg(feature = "schema")]
fn export_field_match_score(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    field: &FieldMetadata,
) -> Option<MetadataMatchScore> {
    let mut best = export_candidate_match_score(
        path,
        explicit_array_segments,
        &field.path,
        &field.path_explicit_array_segments,
    );

    for alias in &field.aliases {
        let empty = BTreeSet::new();
        let alias_explicit_array_segments = field
            .alias_explicit_array_segments
            .get(alias)
            .unwrap_or(&empty);
        let Some(score) = export_candidate_match_score(
            path,
            explicit_array_segments,
            alias,
            alias_explicit_array_segments,
        ) else {
            continue;
        };

        match &mut best {
            Some(best_score) if score > *best_score => *best_score = score,
            None => best = Some(score),
            _ => {}
        }
    }

    best
}

#[cfg(feature = "schema")]
fn export_candidate_match_score(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    candidate: &str,
    candidate_explicit_array_segments: &BTreeSet<usize>,
) -> Option<MetadataMatchScore> {
    let score = metadata_match_score(path, candidate)?;
    explicit_array_intent_matches_export_path(
        path,
        explicit_array_segments,
        candidate,
        candidate_explicit_array_segments,
    )
    .then_some(score)
}

#[cfg(feature = "schema")]
fn explicit_array_intent_matches_export_path(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    candidate: &str,
    candidate_explicit_array_segments: &BTreeSet<usize>,
) -> bool {
    let query_segments = path_segments(path);
    let candidate_segments = path_segments(candidate);

    candidate_explicit_array_segments
        .difference(explicit_array_segments)
        .all(|index| {
            query_segments.get(*index) == Some(&"*") || candidate_segments.get(*index) == Some(&"*")
        })
}
