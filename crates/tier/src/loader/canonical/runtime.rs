use std::collections::BTreeSet;

use serde_json::Value;

use crate::ConfigError;
use crate::metadata::ConfigMetadata;
use crate::path::{ArrayIndexSegment, classify_array_index_segment, normalize_path};

use super::super::Layer;
use super::super::secret_path::SecretPathSpec;

pub(in crate::loader) fn canonicalize_runtime_path(value: &Value, path: &str) -> String {
    try_canonicalize_runtime_path(value, path).unwrap_or_else(|_| normalize_path(path))
}

pub(in crate::loader) fn try_canonicalize_runtime_path(
    value: &Value,
    path: &str,
) -> Result<String, String> {
    try_canonicalize_runtime_path_with_explicit_arrays(value, path, &BTreeSet::new())
}

pub(in crate::loader) fn try_canonicalize_runtime_path_with_explicit_arrays(
    value: &Value,
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
) -> Result<String, String> {
    if path.is_empty() {
        return Ok(String::new());
    }

    let segments = path.split('.').collect::<Vec<_>>();
    let mut current = value;
    let mut canonical = Vec::new();
    let mut index = 0;
    while index < segments.len() {
        let segment = segments[index];
        let explicit_array_segment = explicit_array_segments.contains(&index);
        match current {
            Value::Object(map) if explicit_array_segment => {
                let array_index = explicit_array_index(segment)?;
                canonical.push(array_index.to_string());
                if map.is_empty() {
                    append_remaining_segments(&mut canonical, &segments[index + 1..]);
                    break;
                }
                return Err(format!(
                    "path segment `{segment}` uses array syntax at a known object position"
                ));
            }
            Value::Object(map) => {
                canonical.push(segment.to_owned());
                let Some(next) = map.get(segment) else {
                    append_remaining_segments(&mut canonical, &segments[index + 1..]);
                    break;
                };
                current = next;
            }
            Value::Array(values) => {
                let array_index = match classify_array_index_segment(segment) {
                    ArrayIndexSegment::Index(array_index) => array_index,
                    ArrayIndexSegment::NonNumeric if segment == "*" => {
                        canonical.push(segment.to_owned());
                        append_remaining_segments(&mut canonical, &segments[index + 1..]);
                        break;
                    }
                    ArrayIndexSegment::NonNumeric => {
                        return Err(format!(
                            "array path segment `{segment}` must be an array index or `*` wildcard at this position"
                        ));
                    }
                    ArrayIndexSegment::Invalid(message) => {
                        return Err(format!(
                            "array index segment `{segment}` is invalid at this position: {message}"
                        ));
                    }
                };
                canonical.push(array_index.to_string());
                let Some(next) = values.get(array_index) else {
                    append_remaining_segments(&mut canonical, &segments[index + 1..]);
                    break;
                };
                current = next;
            }
            _ => {
                if explicit_array_segment {
                    let _ = explicit_array_index(segment)?;
                    return Err(format!(
                        "path segment `{segment}` uses array syntax at a known non-container position"
                    ));
                }
                canonical.push(segment.to_owned());
                append_remaining_segments(&mut canonical, &segments[index + 1..]);
                break;
            }
        }
        index += 1;
    }

    Ok(canonical.join("."))
}

fn explicit_array_index(segment: &str) -> Result<usize, String> {
    match classify_array_index_segment(segment) {
        ArrayIndexSegment::Index(array_index) => Ok(array_index),
        ArrayIndexSegment::NonNumeric => Err(format!(
            "array path segment `{segment}` must be an array index at this position"
        )),
        ArrayIndexSegment::Invalid(message) => Err(format!(
            "array index segment `{segment}` is invalid at this position: {message}"
        )),
    }
}

pub(in crate::loader) fn canonicalize_secret_paths_against_layers(
    secret_paths: &BTreeSet<SecretPathSpec>,
    defaults_value: &Value,
    layers: &[Layer],
    metadata: &ConfigMetadata,
) -> Result<BTreeSet<String>, ConfigError> {
    let mut current =
        canonicalize_secret_path_specs_against_value(secret_paths, defaults_value, metadata)?;
    for layer in layers {
        current = canonicalize_secret_path_specs_against_value(&current, &layer.value, metadata)?;
    }
    Ok(current.into_iter().map(SecretPathSpec::into_path).collect())
}

pub(in crate::loader) fn canonicalize_secret_paths_against_value(
    secret_paths: &BTreeSet<SecretPathSpec>,
    value: &Value,
    metadata: &ConfigMetadata,
) -> Result<BTreeSet<String>, ConfigError> {
    Ok(
        canonicalize_secret_path_specs_against_value(secret_paths, value, metadata)?
            .into_iter()
            .map(SecretPathSpec::into_path)
            .collect(),
    )
}

fn canonicalize_secret_path_specs_against_value(
    secret_paths: &BTreeSet<SecretPathSpec>,
    value: &Value,
    metadata: &ConfigMetadata,
) -> Result<BTreeSet<SecretPathSpec>, ConfigError> {
    let mut canonicalized = BTreeSet::new();
    for spec in secret_paths {
        let runtime = try_canonicalize_runtime_path_with_explicit_arrays(
            value,
            spec.path(),
            spec.explicit_array_segments(),
        )
        .map_err(|message| invalid_secret_path(spec.path(), message))?;
        let (path, explicit_array_segments) = metadata
            .canonicalize_alias_path_with_array_segments_for_shape(
                &runtime,
                spec.explicit_array_segments(),
                Some(value),
            )?;
        canonicalized.insert(SecretPathSpec::from_normalized(
            path,
            explicit_array_segments,
        ));
    }
    Ok(canonicalized)
}

pub(in crate::loader) fn try_canonicalize_runtime_path_across_layers_with_explicit_arrays(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    layers: &[Layer],
) -> Result<String, String> {
    let mut current = normalize_path(path);
    for layer in layers {
        current = try_canonicalize_runtime_path_with_explicit_arrays(
            &layer.value,
            &current,
            explicit_array_segments,
        )?;
    }
    Ok(current)
}

fn append_remaining_segments(canonical: &mut Vec<String>, segments: &[&str]) {
    canonical.extend(segments.iter().map(|segment| (*segment).to_owned()));
}

fn invalid_secret_path(path: &str, message: String) -> ConfigError {
    ConfigError::MetadataInvalid {
        path: path.to_owned(),
        message: format!("invalid secret path: {message}"),
    }
}
