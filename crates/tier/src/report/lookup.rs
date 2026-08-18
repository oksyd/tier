use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::path::{
    ExternalPathSegment, canonicalize_path_with_aliases, get_value_at_path, is_array_index_segment,
    normalize_path, parse_array_index_segment, parse_external_path, path_overlaps_pattern,
    render_external_path,
};

use super::model::ResolutionStep;

pub(super) fn normalize_lookup_path(
    path: &str,
    final_value: &Value,
    alias_overrides: &BTreeMap<String, String>,
    traces: &BTreeMap<String, Vec<ResolutionStep>>,
) -> Option<String> {
    let segments = parse_external_path(path).ok()?;
    let normalized = render_external_path(&segments);
    let runtime = canonicalize_runtime_lookup_path(final_value, &segments)?;
    let aliased_runtime = canonicalize_path_with_aliases(&runtime, alias_overrides);
    if traces.contains_key(&aliased_runtime)
        || get_value_at_path(final_value, &aliased_runtime).is_some()
    {
        return Some(aliased_runtime);
    }

    let aliased_normalized = canonicalize_path_with_aliases(&normalized, alias_overrides);
    if traces.contains_key(&aliased_normalized)
        || get_value_at_path(final_value, &aliased_normalized).is_some()
    {
        return Some(aliased_normalized);
    }

    Some(aliased_runtime)
}

pub(super) fn path_overlaps_secret(path: &str, secret_paths: &BTreeSet<String>) -> bool {
    secret_paths
        .iter()
        .any(|secret| path_overlaps_pattern(path, secret))
}

fn canonicalize_runtime_lookup_path(
    value: &Value,
    segments: &[ExternalPathSegment],
) -> Option<String> {
    let mut current = value;
    let mut canonical = Vec::new();

    for (index, segment) in segments.iter().enumerate() {
        match current {
            Value::Object(map) => {
                let ExternalPathSegment::Field(field) = segment else {
                    return None;
                };
                canonical.push(field.clone());
                let Some(next) = map.get(field) else {
                    append_remaining_segments(&mut canonical, &segments[index + 1..]);
                    break;
                };
                current = next;
            }
            Value::Array(values) => {
                let array_index = match segment {
                    ExternalPathSegment::Index(array_index) => array_index.clone(),
                    ExternalPathSegment::Field(field) if is_array_index_segment(field) => {
                        field.clone()
                    }
                    ExternalPathSegment::Field(_) => return None,
                };
                let Ok(array_index) = parse_array_index_segment(&array_index) else {
                    canonical.push(array_index);
                    append_remaining_segments(&mut canonical, &segments[index + 1..]);
                    break;
                };
                canonical.push(array_index.to_string());
                let Some(next) = values.get(array_index) else {
                    append_remaining_segments(&mut canonical, &segments[index + 1..]);
                    break;
                };
                current = next;
            }
            _ => {
                canonical.push(segment.value().to_owned());
                append_remaining_segments(&mut canonical, &segments[index + 1..]);
                break;
            }
        }
    }

    Some(normalize_path(&canonical.join(".")))
}

fn append_remaining_segments(canonical: &mut Vec<String>, segments: &[ExternalPathSegment]) {
    canonical.extend(segments.iter().map(|segment| segment.value().to_owned()));
}
