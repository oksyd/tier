use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::ConfigError;
use crate::metadata::paths::{
    alias_mapping_is_lossless, alias_overlap_sample_path, alias_patterns_are_ambiguous,
    render_metadata_path, validate_metadata_path,
};
use crate::path::{normalize_path, path_segments};

use super::super::{AliasOverrideSpec, ConfigMetadata, FieldMetadata};

impl ConfigMetadata {
    /// Returns explicit path aliases keyed by alias path.
    pub fn alias_overrides(&self) -> Result<BTreeMap<String, String>, ConfigError> {
        Ok(self
            .alias_override_specs()?
            .into_iter()
            .map(|spec| {
                (
                    render_metadata_path(&spec.alias, &spec.alias_explicit_array_segments),
                    render_metadata_path(&spec.canonical, &spec.canonical_explicit_array_segments),
                )
            })
            .collect())
    }

    pub(crate) fn alias_lookup_overrides(&self) -> Result<BTreeMap<String, String>, ConfigError> {
        Ok(self
            .alias_override_specs()?
            .into_iter()
            .map(|spec| (spec.alias, spec.canonical))
            .collect())
    }

    pub(crate) fn alias_override_specs(&self) -> Result<Vec<AliasOverrideSpec>, ConfigError> {
        let mut aliases = BTreeMap::<String, String>::new();
        let mut specs = Vec::new();
        let canonical_paths = self
            .fields
            .iter()
            .map(|field| field.path.clone())
            .collect::<BTreeSet<_>>();

        for field in &self.fields {
            validate_metadata_path(&field.path)?;
            for alias in &field.aliases {
                validate_metadata_path(alias)?;
                validate_alias(field, alias, &canonical_paths)?;
                if let Some(first_path) = aliases.get(alias)
                    && first_path != &field.path
                {
                    return Err(ConfigError::MetadataConflict {
                        kind: "alias",
                        name: alias.clone(),
                        first_path: first_path.clone(),
                        second_path: field.path.clone(),
                    });
                }
                if let Some((other_alias, sample_path)) =
                    ambiguous_alias_overlap(&aliases, alias, &field.path)
                {
                    return Err(ConfigError::MetadataInvalid {
                        path: alias.clone(),
                        message: format!(
                            "alias `{alias}` overlaps ambiguously with `{other_alias}` for concrete path `{sample_path}`"
                        ),
                    });
                }
                aliases.insert(alias.clone(), field.path.clone());
                specs.push(AliasOverrideSpec {
                    alias: alias.clone(),
                    alias_explicit_array_segments: field
                        .alias_explicit_array_segments
                        .get(alias)
                        .cloned()
                        .unwrap_or_default(),
                    canonical: field.path.clone(),
                    canonical_explicit_array_segments: field.path_explicit_array_segments.clone(),
                });
            }
        }
        Ok(specs)
    }

    pub(crate) fn canonicalize_alias_path_with_array_segments(
        &self,
        path: &str,
        explicit_array_segments: &BTreeSet<usize>,
    ) -> Result<(String, BTreeSet<usize>), ConfigError> {
        self.canonicalize_alias_path_with_array_segments_for_shape(
            path,
            explicit_array_segments,
            None,
        )
    }

    pub(crate) fn canonicalize_alias_path_with_array_segments_for_shape(
        &self,
        path: &str,
        explicit_array_segments: &BTreeSet<usize>,
        shape: Option<&Value>,
    ) -> Result<(String, BTreeSet<usize>), ConfigError> {
        let specs = self.alias_override_specs()?;
        Ok(canonicalize_path_with_alias_specs_and_array_segments(
            path,
            explicit_array_segments,
            &specs,
            shape,
        ))
    }

    pub(crate) fn canonicalize_path_with_alias_specs_and_array_segments(
        path: &str,
        explicit_array_segments: &BTreeSet<usize>,
        specs: &[AliasOverrideSpec],
        shape: Option<&Value>,
    ) -> (String, BTreeSet<usize>) {
        canonicalize_path_with_alias_specs_and_array_segments(
            path,
            explicit_array_segments,
            specs,
            shape,
        )
    }
}

pub(crate) fn canonicalize_path_with_alias_specs_and_array_segments(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    specs: &[AliasOverrideSpec],
    shape: Option<&Value>,
) -> (String, BTreeSet<usize>) {
    let normalized = normalize_path(path);
    if normalized.is_empty() || specs.is_empty() {
        return (normalized, explicit_array_segments.clone());
    }

    let Some(rewrite) = best_alias_spec_rewrite(&normalized, explicit_array_segments, specs, shape)
    else {
        return (normalized, explicit_array_segments.clone());
    };
    (rewrite.path, rewrite.array_segments)
}

struct AliasSpecRewrite {
    alias_len: usize,
    specificity: usize,
    path: String,
    array_segments: BTreeSet<usize>,
}

fn best_alias_spec_rewrite(
    normalized: &str,
    explicit_array_segments: &BTreeSet<usize>,
    specs: &[AliasOverrideSpec],
    shape: Option<&Value>,
) -> Option<AliasSpecRewrite> {
    let concrete_segments = path_segments(normalized);
    let mut best = None::<AliasSpecRewrite>;

    for spec in specs {
        let alias_segments = path_segments(&spec.alias);
        if !alias_spec_matches(
            &concrete_segments,
            explicit_array_segments,
            &alias_segments,
            spec,
            shape,
        ) {
            continue;
        }

        let specificity = alias_segments
            .iter()
            .filter(|segment| **segment != "*")
            .count();
        let rewrite = AliasSpecRewrite {
            alias_len: alias_segments.len(),
            specificity,
            path: rewrite_alias_spec_path(&concrete_segments, &alias_segments, &spec.canonical),
            array_segments: rewrite_alias_spec_array_segments(
                explicit_array_segments,
                alias_segments.len(),
                path_segments(&spec.canonical).len(),
                &spec.canonical_explicit_array_segments,
            ),
        };

        match &mut best {
            Some(best)
                if rewrite.alias_len > best.alias_len
                    || (rewrite.alias_len == best.alias_len
                        && rewrite.specificity > best.specificity) =>
            {
                *best = rewrite;
            }
            None => best = Some(rewrite),
            _ => {}
        }
    }

    best
}

fn alias_spec_matches(
    path_segments: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    alias_segments: &[&str],
    spec: &AliasOverrideSpec,
    shape: Option<&Value>,
) -> bool {
    if alias_segments.len() > path_segments.len() {
        return false;
    }

    alias_segments
        .iter()
        .zip(path_segments.iter())
        .enumerate()
        .all(|(index, (expected, actual))| {
            if *expected != "*" && expected != actual {
                return false;
            }

            let actual_explicit = explicit_array_segments.contains(&index);
            let alias_explicit = spec.alias_explicit_array_segments.contains(&index);
            actual_explicit == alias_explicit
                || shape_confirms_canonical_array_segment(shape, &spec.canonical, index)
        })
}

fn shape_confirms_canonical_array_segment(
    shape: Option<&Value>,
    canonical: &str,
    segment_index: usize,
) -> bool {
    let Some(mut current) = shape else {
        return false;
    };
    let canonical_segments = path_segments(canonical);
    if segment_index >= canonical_segments.len() {
        return false;
    }

    for segment in &canonical_segments[..segment_index] {
        match current {
            Value::Object(map) => {
                let Some(next) = map.get(*segment) else {
                    return false;
                };
                current = next;
            }
            Value::Array(values) => {
                let Ok(index) = segment.parse::<usize>() else {
                    return false;
                };
                let Some(next) = values.get(index) else {
                    return false;
                };
                current = next;
            }
            _ => return false,
        }
    }

    current.is_array()
}

fn rewrite_alias_spec_path(
    concrete_segments: &[&str],
    alias_segments: &[&str],
    canonical: &str,
) -> String {
    let canonical_segments = path_segments(canonical);
    let mut rewritten = canonical_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            if *segment == "*" && alias_segments.get(index) == Some(&"*") {
                concrete_segments[index].to_owned()
            } else {
                (*segment).to_owned()
            }
        })
        .collect::<Vec<_>>();
    rewritten.extend(
        concrete_segments[alias_segments.len()..]
            .iter()
            .map(|segment| (*segment).to_owned()),
    );
    normalize_path(&rewritten.join("."))
}

fn rewrite_alias_spec_array_segments(
    explicit_array_segments: &BTreeSet<usize>,
    alias_len: usize,
    canonical_len: usize,
    canonical_explicit_array_segments: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut rewritten = canonical_explicit_array_segments.clone();
    rewritten.extend(
        explicit_array_segments
            .iter()
            .filter(|index| **index >= alias_len)
            .map(|index| canonical_len + (*index - alias_len)),
    );
    rewritten
}

fn validate_alias(
    field: &FieldMetadata,
    alias: &str,
    canonical_paths: &BTreeSet<String>,
) -> Result<(), ConfigError> {
    if alias.is_empty() {
        return Err(ConfigError::MetadataInvalid {
            path: alias.to_owned(),
            message: "aliases cannot target the root path".to_owned(),
        });
    }
    if field.path.is_empty() {
        return Err(ConfigError::MetadataInvalid {
            path: alias.to_owned(),
            message: "aliases cannot rewrite the root path".to_owned(),
        });
    }
    if !alias_mapping_is_lossless(alias, &field.path) {
        return Err(ConfigError::MetadataInvalid {
            path: alias.to_owned(),
            message: format!(
                "alias `{alias}` must preserve wildcard positions and cannot be deeper than canonical path `{}`",
                field.path
            ),
        });
    }
    if canonical_paths.contains(alias) && alias != field.path {
        return Err(ConfigError::MetadataConflict {
            kind: "alias",
            name: alias.to_owned(),
            first_path: alias.to_owned(),
            second_path: field.path.clone(),
        });
    }
    Ok(())
}

fn ambiguous_alias_overlap(
    aliases: &BTreeMap<String, String>,
    alias: &str,
    canonical: &str,
) -> Option<(String, String)> {
    aliases.iter().find_map(|(other_alias, other_canonical)| {
        alias_patterns_are_ambiguous(alias, canonical, other_alias, other_canonical).then(|| {
            (
                other_alias.clone(),
                alias_overlap_sample_path(alias, other_alias),
            )
        })
    })
}
