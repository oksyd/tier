use std::collections::BTreeSet;

use serde_json::Value;

use crate::path::{ExternalPathSegment, normalize_path, parse_array_index_segment};

enum PatchShape<'a> {
    Value(&'a Value),
    Object,
    Array,
}

pub(super) fn parse_patch_path(path: &str) -> Result<(Vec<String>, BTreeSet<usize>), String> {
    let mut explicit_array_segments = BTreeSet::new();
    let segments = crate::path::parse_external_path(path)?
        .into_iter()
        .enumerate()
        .map(|(index, segment)| {
            if matches!(&segment, ExternalPathSegment::Index(_)) {
                explicit_array_segments.insert(index);
            }
            segment.value().to_owned()
        })
        .collect();

    Ok((segments, explicit_array_segments))
}

pub(super) fn canonicalize_patch_path(
    root: &Value,
    segments: &[String],
    explicit_array_segments: &BTreeSet<usize>,
) -> Result<(Vec<String>, BTreeSet<usize>), String> {
    let mut canonical = Vec::with_capacity(segments.len());
    let mut array_segments = BTreeSet::new();
    let mut current = PatchShape::Value(root);
    let mut index = 0;

    while index < segments.len() {
        let segment = &segments[index];
        let is_last = index + 1 == segments.len();
        let next_is_explicit_array = !is_last && explicit_array_segments.contains(&(index + 1));

        match current {
            PatchShape::Value(Value::Object(map)) => {
                canonical.push(segment.clone());
                current = if is_last {
                    PatchShape::Object
                } else if let Some(next) = map.get(segment) {
                    object_child_patch_shape(next, next_is_explicit_array, &segments[index + 1])?
                } else if next_is_explicit_array {
                    PatchShape::Array
                } else {
                    PatchShape::Object
                };
            }
            PatchShape::Value(Value::Array(values)) => {
                let array_index = parse_array_index_segment(segment).map_err(|message| {
                    format!("array path segment `{segment}` is invalid at this position: {message}")
                })?;
                canonical.push(array_index.to_string());
                array_segments.insert(index);
                current = if is_last {
                    PatchShape::Array
                } else if let Some(next) = values.get(array_index) {
                    object_or_array_patch_shape(next, next_is_explicit_array, &segments[index + 1])?
                } else if next_is_explicit_array {
                    PatchShape::Array
                } else {
                    PatchShape::Object
                };
            }
            PatchShape::Value(_) => {
                canonical.extend(segments[index..].iter().cloned());
                break;
            }
            PatchShape::Object => {
                canonical.push(segment.clone());
                current = if is_last {
                    PatchShape::Object
                } else if next_is_explicit_array {
                    PatchShape::Array
                } else {
                    PatchShape::Object
                };
            }
            PatchShape::Array => {
                let array_index = parse_array_index_segment(segment).map_err(|message| {
                    format!("array path segment `{segment}` is invalid at this position: {message}")
                })?;
                canonical.push(array_index.to_string());
                array_segments.insert(index);
                current = if is_last || next_is_explicit_array {
                    PatchShape::Array
                } else {
                    PatchShape::Object
                };
            }
        }

        index += 1;
    }

    Ok((canonical, array_segments))
}

fn object_child_patch_shape<'a>(
    child: &'a Value,
    next_is_explicit_array: bool,
    next_segment: &str,
) -> Result<PatchShape<'a>, String> {
    match child {
        Value::Object(map) if map.is_empty() && next_is_explicit_array => Ok(PatchShape::Array),
        Value::Object(_) if next_is_explicit_array => Err(format!(
            "path segment {next_segment} uses array syntax after an object segment"
        )),
        _ => Ok(PatchShape::Value(child)),
    }
}

fn object_or_array_patch_shape<'a>(
    child: &'a Value,
    next_is_explicit_array: bool,
    next_segment: &str,
) -> Result<PatchShape<'a>, String> {
    match child {
        Value::Object(_) => object_child_patch_shape(child, next_is_explicit_array, next_segment),
        _ => Ok(PatchShape::Value(child)),
    }
}

pub(super) fn patch_indexed_array_container_paths(
    segments: &[String],
    array_segments: &BTreeSet<usize>,
) -> BTreeSet<String> {
    array_segments
        .iter()
        .map(|index| normalize_path(&segments[..*index].join(".")))
        .collect()
}
