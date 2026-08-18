use std::collections::BTreeSet;

use crate::ConfigError;
use crate::path::{
    normalize_path, path_matches_pattern, path_segments, render_path_with_explicit_array_segments,
};

use super::{MetadataMatchScore, MetadataPathSpec};

pub(super) fn normalize_check_path_group_specs<I>(paths: I) -> Option<Vec<MetadataPathSpec>>
where
    I: IntoIterator<Item = String>,
{
    let mut normalized = Vec::new();
    for path in paths {
        let path = normalize_check_path(&path);
        if normalized.contains(&path) {
            continue;
        }
        normalized.push(path);
    }
    (!normalized.is_empty()).then_some(normalized)
}

pub(super) fn normalize_check_path(path: &str) -> MetadataPathSpec {
    let (path, explicit_array_segments) = normalize_metadata_path_with_explicit_arrays(path);
    MetadataPathSpec {
        path,
        explicit_array_segments,
    }
}

pub(super) fn path_spec_to_public_path(spec: &MetadataPathSpec) -> String {
    render_metadata_path(&spec.path, &spec.explicit_array_segments)
}

pub(super) fn render_metadata_path(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
) -> String {
    if try_normalize_metadata_path(path).is_err() {
        return path.to_owned();
    }
    render_path_with_explicit_array_segments(path, explicit_array_segments)
}

pub(super) fn normalize_metadata_path(path: &str) -> String {
    try_normalize_metadata_path(path).unwrap_or_else(|_| path.to_owned())
}

pub(super) fn normalize_metadata_path_with_explicit_arrays(
    path: &str,
) -> (String, BTreeSet<usize>) {
    try_normalize_metadata_path_with_explicit_arrays(path)
        .unwrap_or_else(|_| (path.to_owned(), BTreeSet::new()))
}

pub(super) fn validate_metadata_path(path: &str) -> Result<(), ConfigError> {
    try_normalize_metadata_path(path)
        .map(|_| ())
        .map_err(|message| ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: format!("invalid metadata path: {message}"),
        })
}

pub(super) fn validate_check_path(path: &str) -> Result<(), ConfigError> {
    validate_metadata_path(path)?;
    if normalize_metadata_path(path).is_empty() {
        return Err(ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: "invalid metadata path: cross-field checks cannot use the root path"
                .to_owned(),
        });
    }
    Ok(())
}

pub(super) fn try_normalize_metadata_path(path: &str) -> Result<String, String> {
    try_normalize_metadata_path_with_explicit_arrays(path).map(|(path, _)| path)
}

pub(super) fn try_normalize_metadata_path_with_explicit_arrays(
    path: &str,
) -> Result<(String, BTreeSet<usize>), String> {
    let segments = crate::path::parse_external_path(path)?;
    for segment in &segments {
        let segment = segment.value();
        if segment.contains('*') && segment != "*" {
            return Err("wildcard path segments must be exactly `*`".to_owned());
        }
    }

    let explicit_array_segments = segments
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| {
            matches!(segment, crate::path::ExternalPathSegment::Index(_)).then_some(index)
        })
        .collect();
    Ok((
        crate::path::render_external_path(&segments),
        explicit_array_segments,
    ))
}

pub(super) fn shift_explicit_array_segments(
    segments: &BTreeSet<usize>,
    offset: usize,
) -> BTreeSet<usize> {
    segments.iter().map(|index| index + offset).collect()
}

pub(super) fn join_explicit_array_segments(
    prefix_segments: &BTreeSet<usize>,
    prefix_len: usize,
    suffix_segments: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut joined = prefix_segments.clone();
    joined.extend(shift_explicit_array_segments(suffix_segments, prefix_len));
    joined
}

pub(super) fn metadata_match_score(path: &str, candidate: &str) -> Option<MetadataMatchScore> {
    if candidate != path && !path_matches_pattern(path, candidate) {
        return None;
    }

    let segments = candidate
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let positional_specificity = segments
        .iter()
        .map(|segment| *segment != "*")
        .collect::<Vec<_>>();
    let specificity = positional_specificity
        .iter()
        .filter(|segment| **segment)
        .count();
    Some(MetadataMatchScore {
        segment_count: segments.len(),
        specificity,
        positional_specificity,
    })
}

pub(super) fn alias_mapping_is_lossless(alias: &str, canonical: &str) -> bool {
    let alias_segments = path_segments(alias);
    let canonical_segments = path_segments(canonical);
    if canonical_segments.len() < alias_segments.len() {
        return false;
    }

    for index in 0..alias_segments.len() {
        let alias_wildcard = alias_segments[index] == "*";
        let canonical_wildcard = canonical_segments[index] == "*";
        if alias_wildcard != canonical_wildcard {
            return false;
        }
    }

    !canonical_segments[alias_segments.len()..].contains(&"*")
}

pub(super) fn alias_patterns_are_ambiguous(
    left_alias: &str,
    left_canonical: &str,
    right_alias: &str,
    right_canonical: &str,
) -> bool {
    if alias_rank(left_alias) != alias_rank(right_alias) {
        return false;
    }

    let left_segments = path_segments(left_alias);
    let right_segments = path_segments(right_alias);
    if left_segments.len() != right_segments.len() {
        return false;
    }

    if !left_segments
        .iter()
        .zip(right_segments.iter())
        .all(|(left, right)| *left == "*" || *right == "*" || left == right)
    {
        return false;
    }

    let sample_path = alias_overlap_sample_path(left_alias, right_alias);
    rewrite_alias_sample(&sample_path, left_alias, left_canonical)
        != rewrite_alias_sample(&sample_path, right_alias, right_canonical)
}

fn alias_rank(alias: &str) -> (usize, usize) {
    let segments = path_segments(alias);
    let specificity = segments.iter().filter(|segment| **segment != "*").count();
    (segments.len(), specificity)
}

pub(super) fn alias_overlap_sample_path(left: &str, right: &str) -> String {
    path_segments(left)
        .into_iter()
        .zip(path_segments(right))
        .map(|(left, right)| {
            if left == "*" && right == "*" {
                "item".to_owned()
            } else if left == "*" {
                right.to_owned()
            } else {
                left.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn rewrite_alias_sample(path: &str, alias: &str, canonical: &str) -> String {
    let concrete_segments = path_segments(path);
    let alias_segments = path_segments(alias);
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
