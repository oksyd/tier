use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::ConfigError;
use crate::path::{
    checked_array_len_for_index, concrete_paths_overlap, direct_child_array_index,
    parse_array_index_segment,
};

#[derive(Clone, Copy)]
enum PatchContainer {
    Object,
    Array,
}

impl PatchContainer {
    fn empty_value(self) -> Value {
        match self {
            Self::Object => Value::Object(Map::new()),
            Self::Array => Value::Array(Vec::new()),
        }
    }
}

pub(super) fn record_patch_indexed_array_state(
    current_array_lengths: &mut BTreeMap<String, usize>,
    indexed_array_base_lengths: &mut BTreeMap<String, usize>,
    path: &str,
    indexed_array_container_paths: &BTreeSet<String>,
) {
    for container_path in indexed_array_container_paths {
        let Some(index) = direct_child_array_index(container_path, path) else {
            continue;
        };
        let Some(current_length) = current_array_lengths.get_mut(container_path) else {
            continue;
        };

        indexed_array_base_lengths
            .entry(container_path.clone())
            .or_insert(*current_length);
        if index >= *current_length
            && let Ok(next_length) = checked_array_len_for_index(index)
        {
            *current_length = next_length;
        }
    }
}

pub(super) fn insert_path_with_shape(
    current: &mut Value,
    shape: Option<&Value>,
    segments: &[String],
    array_segments: &BTreeSet<usize>,
    depth: usize,
    value: Value,
) -> Result<(), String> {
    let segment = &segments[depth];
    if segment.is_empty() {
        return Err("configuration path contains an empty segment".to_owned());
    }

    let is_last = depth + 1 == segments.len();
    match current {
        Value::Object(map) => {
            if is_last {
                map.insert(segment.clone(), value);
                return Ok(());
            }

            let shape_child = match shape {
                Some(Value::Object(shape_map)) => shape_map.get(segment),
                _ => None,
            };
            let next_is_array = array_segments.contains(&(depth + 1));
            let next_container =
                patch_next_container(shape_child, next_is_array, &segments[depth + 1])?;
            let child = map
                .entry(segment.clone())
                .or_insert_with(|| next_container.empty_value());
            ensure_patch_container(
                child,
                shape_child,
                next_container,
                next_is_array,
                &segments[depth + 1],
                segment,
            )?;
            insert_path_with_shape(
                child,
                shape_child,
                segments,
                array_segments,
                depth + 1,
                value,
            )
        }
        Value::Array(values) => {
            let index = parse_array_index_segment(segment).map_err(|message| {
                format!("path segment {segment} must be an array index at this position: {message}")
            })?;

            if values.len() <= index {
                values.resize(checked_array_len_for_index(index)?, Value::Null);
            }

            if is_last {
                values[index] = value;
                return Ok(());
            }

            let shape_child = match shape {
                Some(Value::Array(shape_values)) => shape_values.get(index),
                _ => None,
            };
            let next_is_array = array_segments.contains(&(depth + 1));
            let next_container =
                patch_next_container(shape_child, next_is_array, &segments[depth + 1])?;
            if values[index].is_null() {
                values[index] = next_container.empty_value();
            }
            ensure_patch_container(
                &mut values[index],
                shape_child,
                next_container,
                next_is_array,
                &segments[depth + 1],
                segment,
            )?;
            insert_path_with_shape(
                &mut values[index],
                shape_child,
                segments,
                array_segments,
                depth + 1,
                value,
            )
        }
        _ => Err(format!(
            "path segment {segment} conflicts with an existing non-container value"
        )),
    }
}

pub(super) fn claim_patch_path(
    layer_name: &str,
    path: &str,
    claimed_paths: &mut BTreeSet<String>,
) -> Result<(), ConfigError> {
    for existing_path in claimed_paths.iter() {
        if existing_path == path {
            return Err(ConfigError::InvalidPatch {
                name: layer_name.to_owned(),
                path: path.to_owned(),
                message: format!("duplicate patch path `{path}`"),
            });
        }

        if concrete_paths_overlap(existing_path, path) {
            return Err(ConfigError::InvalidPatch {
                name: layer_name.to_owned(),
                path: path.to_owned(),
                message: format!("conflicting patch paths `{existing_path}` and `{path}` overlap"),
            });
        }
    }

    claimed_paths.insert(path.to_owned());
    Ok(())
}

fn patch_next_container(
    shape_child: Option<&Value>,
    next_is_array: bool,
    next_segment: &str,
) -> Result<PatchContainer, String> {
    match shape_child {
        Some(Value::Object(map)) if map.is_empty() && next_is_array => Ok(PatchContainer::Array),
        Some(Value::Object(_)) if next_is_array => Err(format!(
            "path segment {next_segment} uses array syntax after an object segment"
        )),
        Some(Value::Object(_)) => Ok(PatchContainer::Object),
        Some(Value::Array(_)) => {
            parse_array_index_segment(next_segment).map_err(|message| {
                format!(
                    "path segment {next_segment} must be an array index at this position: {message}"
                )
            })?;
            Ok(PatchContainer::Array)
        }
        Some(_) => Err(format!(
            "path segment {next_segment} conflicts with an existing non-container value"
        )),
        None => Ok(if next_is_array {
            PatchContainer::Array
        } else {
            PatchContainer::Object
        }),
    }
}

fn ensure_patch_container(
    child: &mut Value,
    shape_child: Option<&Value>,
    expected: PatchContainer,
    next_is_array: bool,
    next_segment: &str,
    segment: &str,
) -> Result<(), String> {
    if matches!(expected, PatchContainer::Array)
        && matches!(child, Value::Object(map) if map.is_empty())
        && matches!(shape_child, Some(Value::Object(map)) if map.is_empty())
        && next_is_array
    {
        *child = Value::Array(Vec::new());
    }

    match (child, expected) {
        (Value::Object(_), PatchContainer::Object) | (Value::Array(_), PatchContainer::Array) => {
            Ok(())
        }
        (Value::Object(_), PatchContainer::Array) if next_is_array => Err(format!(
            "path segment {next_segment} uses array syntax after object segment {segment}"
        )),
        _ => Err(format!(
            "path segment {segment} conflicts with an existing non-container value"
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::claim_patch_path;

    #[test]
    fn patch_path_claims_reject_segment_aware_overlaps() {
        let mut claimed = BTreeSet::from(["users".to_owned()]);

        assert!(claim_patch_path("patch", "users.0.name", &mut claimed).is_err());
        assert!(claim_patch_path("patch", "users_profile.0", &mut claimed).is_ok());
    }

    #[test]
    fn patch_path_claims_reject_root_overlap() {
        let mut claimed = BTreeSet::from(["users".to_owned()]);

        assert!(claim_patch_path("patch", "", &mut claimed).is_err());
    }
}
