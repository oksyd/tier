use serde_json::Value;

use crate::path::{join_path, normalize_path, parse_array_index_segment};

pub(super) fn collect_matching_values<'a>(
    value: &'a Value,
    path: &str,
) -> Vec<(String, &'a Value)> {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        return Vec::new();
    }

    let segments = normalized.split('.').collect::<Vec<_>>();
    let mut matches = Vec::new();
    collect_matching_values_recursive(value, "", &segments, 0, &mut matches);
    matches
}

pub(super) fn bind_required_paths(
    trigger_pattern: &str,
    matched_path: &str,
    requires: &[String],
) -> Option<Vec<String>> {
    let bindings = wildcard_bindings(trigger_pattern, matched_path)?;
    Some(
        requires
            .iter()
            .map(|path| apply_wildcard_bindings(path, &bindings))
            .collect(),
    )
}

pub(super) fn present_paths(value: &Value, paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| path_is_present(value, path))
        .cloned()
        .collect()
}

pub(super) fn missing_paths(value: &Value, paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| !path_is_present(value, path))
        .cloned()
        .collect()
}

pub(super) fn is_present_value(value: &Value) -> bool {
    !matches!(value, Value::Null)
}

fn wildcard_bindings(pattern: &str, matched_path: &str) -> Option<Vec<String>> {
    let pattern_segments = pattern
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let path_segments = matched_path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if pattern_segments.len() != path_segments.len() {
        return None;
    }

    let mut bindings = Vec::new();
    for (expected, actual) in pattern_segments.iter().zip(path_segments.iter()) {
        if *expected == "*" {
            bindings.push((*actual).to_owned());
        } else if expected != actual {
            return None;
        }
    }

    Some(bindings)
}

fn apply_wildcard_bindings(pattern: &str, bindings: &[String]) -> String {
    let mut binding_index = 0;
    pattern
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            if segment == "*" {
                let resolved = bindings
                    .get(binding_index)
                    .cloned()
                    .unwrap_or_else(|| "*".to_owned());
                binding_index += 1;
                resolved
            } else {
                segment.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn collect_matching_values_recursive<'a>(
    value: &'a Value,
    current: &str,
    segments: &[&str],
    index: usize,
    matches: &mut Vec<(String, &'a Value)>,
) {
    if index == segments.len() {
        matches.push((current.to_owned(), value));
        return;
    }

    let segment = segments[index];
    match (segment, value) {
        ("*", Value::Object(map)) => {
            for (key, child) in map {
                let next = join_path(current, key);
                collect_matching_values_recursive(child, &next, segments, index + 1, matches);
            }
        }
        ("*", Value::Array(values)) => {
            for (child_index, child) in values.iter().enumerate() {
                let next = join_path(current, &child_index.to_string());
                collect_matching_values_recursive(child, &next, segments, index + 1, matches);
            }
        }
        (_, Value::Object(map)) => {
            if let Some(child) = map.get(segment) {
                let next = join_path(current, segment);
                collect_matching_values_recursive(child, &next, segments, index + 1, matches);
            }
        }
        (_, Value::Array(values)) => {
            if let Ok(child_index) = parse_array_index_segment(segment)
                && let Some(child) = values.get(child_index)
            {
                let next = join_path(current, segment);
                collect_matching_values_recursive(child, &next, segments, index + 1, matches);
            }
        }
        _ => {}
    }
}

fn path_is_present(value: &Value, path: &str) -> bool {
    collect_matching_values(value, path)
        .iter()
        .any(|(_, value)| is_present_value(value))
}
