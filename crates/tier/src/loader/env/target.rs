use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::metadata::EnvOverrideSpec;
use crate::path::{concrete_paths_overlap, parse_array_index_segment, path_segments};
use crate::{ConfigError, ConfigMetadata};

use super::super::Layer;
use super::super::canonical::{
    try_canonicalize_runtime_path_across_layers_with_explicit_arrays,
    try_canonicalize_runtime_path_with_explicit_arrays,
};
use super::super::de::insert_path_with_shape_and_explicit_arrays;
use super::super::path::{
    invalid_concrete_path_segment, try_normalize_external_path,
    try_normalize_external_path_with_explicit_arrays,
};
use super::binding::EnvBinding;

pub(super) fn validate_binding_names(
    bindings: &BTreeMap<String, EnvBinding>,
) -> Result<(), ConfigError> {
    for (name, binding) in bindings {
        if name.is_empty() {
            return Err(ConfigError::InvalidEnv {
                name: name.clone(),
                path: binding.path.clone(),
                message: "environment variable names cannot be empty".to_owned(),
            });
        }
    }

    Ok(())
}

pub(super) fn validate_binding_paths(
    bindings: &BTreeMap<String, EnvBinding>,
    metadata: &ConfigMetadata,
    runtime_layers: &[Layer],
    runtime_shape: &Value,
) -> Result<(), ConfigError> {
    for (name, binding) in bindings {
        let (path, explicit_array_segments) =
            canonical_env_target_path_for_validation(name, &binding.path, metadata, runtime_shape)?;
        let path = canonicalize_env_path_across_layers(
            name,
            &binding.path,
            &path,
            &explicit_array_segments,
            runtime_layers,
        )?;
        validate_runtime_env_target_shape(
            name,
            &binding.path,
            &path,
            &explicit_array_segments,
            runtime_shape,
        )?;
    }
    Ok(())
}

pub(super) fn validate_binding_override_conflicts(
    bindings: &BTreeMap<String, EnvBinding>,
    env_overrides: &BTreeMap<String, EnvOverrideSpec>,
    metadata: &ConfigMetadata,
    runtime_layers: &[Layer],
    runtime_shape: &Value,
) -> Result<(), ConfigError> {
    for (name, binding) in bindings {
        let Some(metadata_spec) = env_overrides.get(name) else {
            continue;
        };

        let (binding_path, binding_explicit_array_segments) =
            canonical_env_target_path_for_validation(name, &binding.path, metadata, runtime_shape)?;
        let binding_path = canonicalize_env_path_across_layers(
            name,
            &binding.path,
            &binding_path,
            &binding_explicit_array_segments,
            runtime_layers,
        )?;
        let metadata_path = canonicalize_env_path_across_layers(
            name,
            &metadata_spec.path,
            &metadata_spec.path,
            &metadata_spec.explicit_array_segments,
            runtime_layers,
        )?;
        let compatible_array_intent = explicit_array_intent_is_equivalent(
            &binding_path,
            &binding_explicit_array_segments,
            &metadata_spec.explicit_array_segments,
            runtime_shape,
        );
        if binding_path != metadata_path || !compatible_array_intent {
            let message = if binding_path == metadata_path {
                format!(
                    "conflicting environment bindings target `{binding_path}` with incompatible array syntax intent via EnvSource and metadata"
                )
            } else {
                format!(
                    "conflicting environment bindings target `{binding_path}` via EnvSource and `{metadata_path}` via metadata"
                )
            };
            return Err(ConfigError::InvalidEnv {
                name: name.clone(),
                path: binding.path.clone(),
                message,
            });
        }
    }

    Ok(())
}

pub(super) fn canonicalize_runtime_env_target_path(
    name: &str,
    path: &str,
    metadata: &ConfigMetadata,
    runtime_layers: &[Layer],
    current_root: &Value,
    runtime_shape: &Value,
) -> Result<String, ConfigError> {
    let (_, explicit_array_segments) = try_normalize_external_path_with_explicit_arrays(path)
        .map_err(|message| ConfigError::InvalidEnv {
            name: name.to_owned(),
            path: path.to_owned(),
            message,
        })?;
    let (path, _) = canonicalize_runtime_env_target_path_with_explicit_arrays(
        name,
        path,
        &explicit_array_segments,
        metadata,
        runtime_layers,
        current_root,
        runtime_shape,
    )?;
    Ok(path)
}

pub(super) fn canonicalize_runtime_env_target_path_with_explicit_arrays(
    name: &str,
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &ConfigMetadata,
    runtime_layers: &[Layer],
    current_root: &Value,
    runtime_shape: &Value,
) -> Result<(String, BTreeSet<usize>), ConfigError> {
    let original_path = path;
    let (canonical_path, explicit_array_segments) = canonical_env_target_path_with_explicit_arrays(
        name,
        original_path,
        explicit_array_segments,
        metadata,
        Some(runtime_shape),
    )?;
    let canonical_path = canonicalize_env_path_across_layers(
        name,
        original_path,
        &canonical_path,
        &explicit_array_segments,
        runtime_layers,
    )?;
    let canonical_path = try_canonicalize_runtime_path_with_explicit_arrays(
        current_root,
        &canonical_path,
        &explicit_array_segments,
    )
    .map_err(|message| ConfigError::InvalidEnv {
        name: name.to_owned(),
        path: original_path.to_owned(),
        message: format!("invalid environment binding path: {message}"),
    })?;
    Ok((canonical_path, explicit_array_segments))
}

pub(super) fn claim_env_path(
    name: &str,
    path: &str,
    claimed_paths: &mut BTreeMap<String, String>,
) -> Result<(), ConfigError> {
    for (existing_path, existing_name) in claimed_paths.iter() {
        if existing_name == name {
            continue;
        }

        if existing_path == path {
            return Err(ConfigError::InvalidEnv {
                name: name.to_owned(),
                path: path.to_owned(),
                message: format!(
                    "conflicting environment variables `{existing_name}` and `{name}` both target `{path}`"
                ),
            });
        }

        if concrete_paths_overlap(existing_path, path) {
            return Err(ConfigError::InvalidEnv {
                name: name.to_owned(),
                path: path.to_owned(),
                message: format!(
                    "conflicting environment variables `{existing_name}` and `{name}` target overlapping configuration paths `{existing_path}` and `{path}`"
                ),
            });
        }
    }

    claimed_paths.insert(path.to_owned(), name.to_owned());
    Ok(())
}

fn canonicalize_env_path_across_layers(
    name: &str,
    original_path: &str,
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    runtime_layers: &[Layer],
) -> Result<String, ConfigError> {
    try_canonicalize_runtime_path_across_layers_with_explicit_arrays(
        path,
        explicit_array_segments,
        runtime_layers,
    )
    .map_err(|message| ConfigError::InvalidEnv {
        name: name.to_owned(),
        path: original_path.to_owned(),
        message: format!("invalid environment binding path: {message}"),
    })
}

fn canonical_env_target_path_for_validation(
    name: &str,
    path: &str,
    metadata: &ConfigMetadata,
    runtime_shape: &Value,
) -> Result<(String, BTreeSet<usize>), ConfigError> {
    let (normalized, explicit_array_segments) =
        try_normalize_external_path_with_explicit_arrays(path).map_err(|message| {
            ConfigError::InvalidEnv {
                name: name.to_owned(),
                path: path.to_owned(),
                message,
            }
        })?;
    validate_normalized_env_target_path(name, path, &normalized)?;
    metadata
        .canonicalize_alias_path_with_array_segments_for_shape(
            &normalized,
            &explicit_array_segments,
            Some(runtime_shape),
        )
        .map_err(|error| metadata_alias_error_to_env_error(name, path, error))
}

fn canonical_env_target_path_with_explicit_arrays(
    name: &str,
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &ConfigMetadata,
    runtime_shape: Option<&Value>,
) -> Result<(String, BTreeSet<usize>), ConfigError> {
    let normalized =
        try_normalize_external_path(path).map_err(|message| ConfigError::InvalidEnv {
            name: name.to_owned(),
            path: path.to_owned(),
            message,
        })?;
    validate_normalized_env_target_path(name, path, &normalized)?;
    metadata
        .canonicalize_alias_path_with_array_segments_for_shape(
            &normalized,
            explicit_array_segments,
            runtime_shape,
        )
        .map_err(|error| metadata_alias_error_to_env_error(name, path, error))
}

fn metadata_alias_error_to_env_error(name: &str, path: &str, error: ConfigError) -> ConfigError {
    match error {
        ConfigError::MetadataInvalid { message, .. } => ConfigError::InvalidEnv {
            name: name.to_owned(),
            path: path.to_owned(),
            message,
        },
        other => other,
    }
}

fn validate_normalized_env_target_path(
    name: &str,
    original_path: &str,
    normalized: &str,
) -> Result<(), ConfigError> {
    if normalized.is_empty() {
        return Err(ConfigError::InvalidEnv {
            name: name.to_owned(),
            path: original_path.to_owned(),
            message: "environment binding path cannot be empty".to_owned(),
        });
    }
    if let Some((segment, message)) = invalid_concrete_path_segment(normalized) {
        return Err(ConfigError::InvalidEnv {
            name: name.to_owned(),
            path: original_path.to_owned(),
            message: format!("environment binding path segment `{segment}` is invalid: {message}"),
        });
    }
    Ok(())
}

fn explicit_array_intent_is_equivalent(
    path: &str,
    left: &BTreeSet<usize>,
    right: &BTreeSet<usize>,
    runtime_shape: &Value,
) -> bool {
    left.symmetric_difference(right)
        .all(|index| shape_confirms_array_segment(runtime_shape, path, *index))
}

fn shape_confirms_array_segment(shape: &Value, path: &str, segment_index: usize) -> bool {
    let segments = path_segments(path);
    if segment_index >= segments.len() {
        return false;
    }

    let mut current = shape;
    for segment in &segments[..segment_index] {
        match current {
            Value::Object(map) => {
                let Some(next) = map.get(*segment) else {
                    return false;
                };
                current = next;
            }
            Value::Array(values) => {
                let Ok(index) = parse_array_index_segment(segment) else {
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

fn validate_runtime_env_target_shape(
    name: &str,
    original_path: &str,
    canonical_path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    runtime_shape: &Value,
) -> Result<(), ConfigError> {
    let segments = canonical_path.split('.').collect::<Vec<_>>();
    let mut probe = Value::Object(serde_json::Map::new());
    insert_path_with_shape_and_explicit_arrays(
        &mut probe,
        Some(runtime_shape),
        &segments,
        explicit_array_segments,
        Value::Null,
    )
    .map_err(|message| ConfigError::InvalidEnv {
        name: name.to_owned(),
        path: original_path.to_owned(),
        message: format!("invalid environment binding path: {message}"),
    })
}
