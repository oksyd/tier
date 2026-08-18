use std::collections::{BTreeMap, BTreeSet};

use crate::path::path_segments;

use super::paths::{
    join_explicit_array_segments, try_normalize_metadata_path_with_explicit_arrays,
};
use super::{ConfigMetadata, FieldMetadata, MetadataPathSpec};

#[derive(Clone)]
struct PrefixSpec {
    path: String,
    explicit_array_segments: BTreeSet<usize>,
}

/// Prefixes child metadata paths with a parent field name.
#[must_use]
pub fn prefixed_metadata(
    prefix: &str,
    prefix_aliases: Vec<String>,
    metadata: ConfigMetadata,
) -> ConfigMetadata {
    let prefix = normalize_prefix(prefix);
    if prefix.path.is_empty() {
        return metadata;
    }
    let prefix_aliases = prefix_aliases
        .into_iter()
        .map(|alias| normalize_prefix(&alias))
        .collect::<Vec<_>>();

    let mut prefixed = ConfigMetadata::from_fields(metadata.fields.into_iter().map(|field| {
        let canonical_suffix = field.path.clone();
        let prefix_segment_count = prefix.segment_count();
        let alias_suffixes = if field.aliases.is_empty() {
            vec![canonical_suffix.clone()]
        } else {
            let mut suffixes = vec![canonical_suffix.clone()];
            suffixes.extend(field.aliases.iter().cloned());
            suffixes
        };

        let path = if canonical_suffix.is_empty() {
            prefix.path.clone()
        } else {
            format!("{}.{}", prefix.path, canonical_suffix)
        };

        let mut aliases = field
            .aliases
            .iter()
            .map(|alias| {
                if alias.is_empty() {
                    prefix.path.clone()
                } else {
                    format!("{}.{}", prefix.path, alias)
                }
            })
            .collect::<Vec<_>>();

        for prefix_alias in &prefix_aliases {
            if canonical_suffix.is_empty() {
                aliases.push(prefix_alias.path.clone());
                continue;
            }
            for suffix in &alias_suffixes {
                if prefix_alias.path.is_empty() {
                    aliases.push(suffix.clone());
                } else {
                    aliases.push(format!("{}.{suffix}", prefix_alias.path));
                }
            }
        }

        let path_explicit_array_segments = join_explicit_array_segments(
            &prefix.explicit_array_segments,
            prefix_segment_count,
            &field.path_explicit_array_segments,
        );
        let alias_explicit_array_segments =
            prefix_alias_explicit_array_segments(&prefix, &prefix_aliases, &field);

        FieldMetadata {
            path,
            aliases,
            path_explicit_array_segments,
            alias_explicit_array_segments,
            ..field
        }
    }));
    prefixed.extend_check_specs(
        metadata
            .check_specs
            .into_iter()
            .filter_map(|check| check.prefixed(&prefix.as_path_spec())),
    );
    prefixed
}

impl PrefixSpec {
    fn segment_count(&self) -> usize {
        path_segments(&self.path).len()
    }

    fn as_path_spec(&self) -> MetadataPathSpec {
        MetadataPathSpec {
            path: self.path.clone(),
            explicit_array_segments: self.explicit_array_segments.clone(),
        }
    }
}

fn normalize_prefix(prefix: &str) -> PrefixSpec {
    if prefix.is_empty() {
        return PrefixSpec {
            path: String::new(),
            explicit_array_segments: BTreeSet::new(),
        };
    }

    let (path, explicit_array_segments) = try_normalize_metadata_path_with_explicit_arrays(prefix)
        .ok()
        .filter(|(normalized, _)| !normalized.is_empty())
        .unwrap_or_else(|| (prefix.to_owned(), BTreeSet::new()));
    PrefixSpec {
        path,
        explicit_array_segments,
    }
}

fn prefix_alias_explicit_array_segments(
    prefix: &PrefixSpec,
    prefix_aliases: &[PrefixSpec],
    field: &FieldMetadata,
) -> BTreeMap<String, BTreeSet<usize>> {
    let prefix_segment_count = prefix.segment_count();
    let mut explicit = BTreeMap::new();
    let empty_segments = BTreeSet::new();

    for alias in &field.aliases {
        let segments = field
            .alias_explicit_array_segments
            .get(alias)
            .unwrap_or(&empty_segments);
        let prefixed_alias = if alias.is_empty() {
            prefix.path.clone()
        } else {
            format!("{}.{alias}", prefix.path)
        };
        insert_explicit_array_segments(
            &mut explicit,
            prefixed_alias,
            join_explicit_array_segments(
                &prefix.explicit_array_segments,
                prefix_segment_count,
                segments,
            ),
        );
    }

    let alias_suffixes = if field.aliases.is_empty() {
        vec![field.path.clone()]
    } else {
        let mut suffixes = vec![field.path.clone()];
        suffixes.extend(field.aliases.iter().cloned());
        suffixes
    };

    for prefix_alias in prefix_aliases {
        let prefix_alias_segment_count = prefix_alias.segment_count();
        for suffix in &alias_suffixes {
            let source_segments = if suffix == &field.path {
                &field.path_explicit_array_segments
            } else {
                field
                    .alias_explicit_array_segments
                    .get(suffix)
                    .unwrap_or(&empty_segments)
            };
            let alias = if prefix_alias.path.is_empty() {
                suffix.clone()
            } else if suffix.is_empty() {
                prefix_alias.path.clone()
            } else {
                format!("{}.{suffix}", prefix_alias.path)
            };
            insert_explicit_array_segments(
                &mut explicit,
                alias,
                join_explicit_array_segments(
                    &prefix_alias.explicit_array_segments,
                    prefix_alias_segment_count,
                    source_segments,
                ),
            );
        }
    }

    explicit
}

fn insert_explicit_array_segments(
    explicit: &mut BTreeMap<String, BTreeSet<usize>>,
    path: String,
    segments: BTreeSet<usize>,
) {
    if !segments.is_empty() {
        explicit.insert(path, segments);
    }
}
