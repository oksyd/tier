use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::ConfigMetadata;
use crate::error::{ConfigError, UnknownField};
use crate::path::{collect_paths, path_is_at_or_below, path_matches_pattern};
use crate::report::ConfigReport;

use super::suggest::{best_path_suggestion, find_source_for_unknown_path};
use crate::loader::path::normalize_external_path;

pub(in crate::loader) fn collect_known_paths<T>(config: &T) -> Result<BTreeSet<String>, ConfigError>
where
    T: Serialize,
{
    let value = serde_json::to_value(config)?;
    Ok(collect_known_paths_from_value(&value))
}

pub(in crate::loader) fn collect_known_paths_from_value(value: &Value) -> BTreeSet<String> {
    let mut paths = Vec::new();
    collect_paths(value, "", &mut paths);
    paths.into_iter().collect()
}

pub(in crate::loader) fn collect_suggestion_paths(
    metadata: &ConfigMetadata,
    known_paths: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let mut candidates = BTreeMap::new();

    for field in metadata.fields() {
        candidates.insert(field.path.clone(), field.path.clone());
        for alias in &field.aliases {
            candidates.insert(alias.clone(), field.path.clone());
        }
    }

    for path in known_paths {
        candidates
            .entry(path.clone())
            .or_insert_with(|| path.clone());
    }

    candidates
}

pub(in crate::loader) fn collect_unknown_fields_from_metadata_scope(
    value: &Value,
    metadata: &ConfigMetadata,
    suggestion_paths: &BTreeMap<String, String>,
    report: &ConfigReport,
    scope: Option<String>,
) -> Vec<UnknownField> {
    let patterns = metadata_pattern_paths(metadata);
    let mut paths = Vec::new();
    collect_paths(value, "", &mut paths);
    paths.sort();
    paths.dedup();

    paths
        .into_iter()
        .filter(|path| path_is_in_scope(path, scope.as_deref()))
        .filter(|path| !path_is_covered_by_patterns(path, &patterns))
        .map(|path| {
            let source = find_source_for_unknown_path(report, &path);
            let suggestion = best_path_suggestion(&path, suggestion_paths);
            UnknownField::new(path)
                .with_source(source)
                .with_suggestion(suggestion)
        })
        .collect()
}

pub(in crate::loader) fn error_path_for_scope(error: &ConfigError) -> Option<&str> {
    match error {
        ConfigError::Deserialize { path, .. } => Some(path.as_str()),
        _ => None,
    }
}

pub(in crate::loader) fn deserialize_error_scope(path: Option<&str>) -> Option<String> {
    let normalized = path.map(normalize_external_path)?;
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn metadata_pattern_paths(metadata: &ConfigMetadata) -> Vec<String> {
    let mut patterns = Vec::new();
    for field in metadata.fields() {
        patterns.push(field.path.clone());
        patterns.extend(field.aliases.iter().cloned());
    }
    patterns.sort();
    patterns.dedup();
    patterns
}

fn path_is_in_scope(path: &str, scope: Option<&str>) -> bool {
    scope.is_none_or(|scope| path_is_at_or_below(path, scope))
}

fn path_is_covered_by_patterns(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        path_matches_pattern(path, pattern) || path_is_prefix_of_pattern(path, pattern)
    })
}

fn path_is_prefix_of_pattern(prefix: &str, pattern: &str) -> bool {
    let prefix_segments = prefix
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let pattern_segments = pattern
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    prefix_segments.len() <= pattern_segments.len()
        && prefix_segments
            .iter()
            .zip(pattern_segments.iter())
            .all(|(actual, expected)| *expected == "*" || actual == expected)
}
