use std::collections::BTreeMap;

use serde_json::Value;

use crate::metadata::AliasOverrideSpec;
use crate::path::{collect_paths, path_starts_with_pattern};
use crate::{ConfigError, ConfigMetadata, FieldMetadata};

use super::super::Layer;
use super::checks::{
    canonicalize_check_against_layers, canonicalize_check_against_value,
    canonicalize_check_with_alias_specs,
};
use super::runtime::try_canonicalize_runtime_path_across_layers_with_explicit_arrays;
use super::runtime::try_canonicalize_runtime_path_with_explicit_arrays;

pub(in crate::loader) fn canonicalize_alias_specs_against_value(
    metadata: &ConfigMetadata,
    value: &Value,
) -> Result<Vec<AliasOverrideSpec>, ConfigError> {
    let aliases = metadata.alias_override_specs()?;
    let mut canonicalized = BTreeMap::new();
    let mut specs = Vec::new();

    for spec in aliases {
        let alias = match canonicalize_metadata_path_against_value_with_explicit_arrays(
            &spec.alias,
            &spec.alias_explicit_array_segments,
            value,
        ) {
            Ok(alias) => alias,
            Err(_) if !alias_matches_value(value, &spec.alias) => continue,
            Err(error) => return Err(error),
        };
        let canonical = canonicalize_metadata_path_against_value_with_explicit_arrays(
            &spec.canonical,
            &spec.canonical_explicit_array_segments,
            value,
        )?;
        if alias == canonical {
            continue;
        }
        if let Some(first_path) = canonicalized.insert(alias.clone(), canonical.clone())
            && first_path != canonical
        {
            return Err(ConfigError::MetadataConflict {
                kind: "alias",
                name: alias,
                first_path,
                second_path: canonical,
            });
        }
        specs.push(AliasOverrideSpec {
            alias,
            alias_explicit_array_segments: spec.alias_explicit_array_segments,
            canonical,
            canonical_explicit_array_segments: spec.canonical_explicit_array_segments,
        });
    }

    Ok(specs)
}

fn alias_matches_value(value: &Value, alias: &str) -> bool {
    let mut paths = Vec::new();
    collect_paths(value, "", &mut paths);
    paths
        .iter()
        .any(|path| path_starts_with_pattern(path, alias))
}

pub(in crate::loader) fn canonicalize_metadata_against_layers(
    metadata: &ConfigMetadata,
    layers: &[Layer],
) -> Result<ConfigMetadata, ConfigError> {
    let fields = metadata
        .fields()
        .iter()
        .cloned()
        .map(|mut field| {
            field.try_map_paths(|path, explicit_array_segments| {
                canonicalize_metadata_path_against_layers_with_explicit_arrays(
                    path,
                    explicit_array_segments,
                    layers,
                )
            })?;
            Ok(field)
        })
        .collect::<Result<Vec<_>, ConfigError>>()?;
    validate_canonical_explicit_env_targets(&fields)?;

    let mut resolved = ConfigMetadata::new();
    resolved.extend_fields(fields);
    let aliases = resolved.alias_override_specs()?;
    let checks = metadata
        .check_specs()
        .iter()
        .cloned()
        .map(|check| {
            Ok(canonicalize_check_with_alias_specs(
                canonicalize_check_against_layers(check, layers)?,
                &aliases,
                None,
            ))
        })
        .collect::<Result<Vec<_>, ConfigError>>()?;
    resolved.extend_check_specs(checks);
    Ok(resolved)
}

pub(in crate::loader) fn canonicalize_metadata_against_value(
    metadata: &ConfigMetadata,
    value: &Value,
) -> Result<ConfigMetadata, ConfigError> {
    let fields = metadata
        .fields()
        .iter()
        .cloned()
        .map(|mut field| {
            field.try_map_paths(|path, explicit_array_segments| {
                canonicalize_metadata_path_against_value_with_explicit_arrays(
                    path,
                    explicit_array_segments,
                    value,
                )
            })?;
            Ok(field)
        })
        .collect::<Result<Vec<_>, ConfigError>>()?;
    validate_canonical_explicit_env_targets(&fields)?;

    let mut resolved = ConfigMetadata::new();
    resolved.extend_fields(fields);
    let aliases = resolved.alias_override_specs()?;
    let checks = metadata
        .check_specs()
        .iter()
        .cloned()
        .map(|check| {
            Ok(canonicalize_check_with_alias_specs(
                canonicalize_check_against_value(check, value)?,
                &aliases,
                Some(value),
            ))
        })
        .collect::<Result<Vec<_>, ConfigError>>()?;
    resolved.extend_check_specs(checks);
    Ok(resolved)
}

pub(in crate::loader::canonical) fn canonicalize_metadata_path_against_layers_with_explicit_arrays(
    path: &str,
    explicit_array_segments: &std::collections::BTreeSet<usize>,
    layers: &[Layer],
) -> Result<String, ConfigError> {
    try_canonicalize_runtime_path_across_layers_with_explicit_arrays(
        path,
        explicit_array_segments,
        layers,
    )
    .map_err(|message| invalid_runtime_metadata_path(path, message))
}

pub(in crate::loader::canonical) fn canonicalize_metadata_path_against_value_with_explicit_arrays(
    path: &str,
    explicit_array_segments: &std::collections::BTreeSet<usize>,
    value: &Value,
) -> Result<String, ConfigError> {
    try_canonicalize_runtime_path_with_explicit_arrays(value, path, explicit_array_segments)
        .map_err(|message| invalid_runtime_metadata_path(path, message))
}

fn invalid_runtime_metadata_path(path: &str, message: String) -> ConfigError {
    ConfigError::MetadataInvalid {
        path: path.to_owned(),
        message: format!("invalid metadata path: {message}"),
    }
}

fn validate_canonical_explicit_env_targets(fields: &[FieldMetadata]) -> Result<(), ConfigError> {
    let mut targets = BTreeMap::<String, String>::new();
    for field in fields {
        let Some(env) = &field.env else {
            continue;
        };
        if let Some(first_env) = targets.insert(field.path.clone(), env.clone())
            && first_env != *env
        {
            return Err(ConfigError::MetadataConflict {
                kind: "environment override target",
                name: field.path.clone(),
                first_path: first_env,
                second_path: env.clone(),
            });
        }
    }
    Ok(())
}
