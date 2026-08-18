use std::collections::{BTreeMap, BTreeSet};

use crate::ConfigMetadata;
use crate::error::ConfigError;
use crate::metadata::AliasOverrideSpec;
use crate::path::normalize_path;
use serde_json::Value;

use super::Layer;

mod checks;
mod metadata;
mod runtime;
mod value;

use self::metadata::canonicalize_alias_specs_against_value;
pub(super) use self::metadata::{
    canonicalize_metadata_against_layers, canonicalize_metadata_against_value,
};
pub(super) use self::runtime::{
    canonicalize_runtime_path, canonicalize_secret_paths_against_layers,
    canonicalize_secret_paths_against_value,
    try_canonicalize_runtime_path_across_layers_with_explicit_arrays,
    try_canonicalize_runtime_path_with_explicit_arrays,
};
pub(super) use self::value::canonicalize_value_paths;

use self::value::canonicalize_value_paths_with_alias_specs;

pub(super) fn canonicalize_layer_paths(
    layer: Layer,
    metadata: &ConfigMetadata,
    shape: &Value,
) -> Result<Layer, ConfigError> {
    let Layer {
        trace,
        value: raw_value,
        entries,
        coercible_string_paths,
        indexed_array_paths,
        indexed_array_base_lengths,
        direct_array_paths,
    } = layer;
    let alias_specs = canonicalize_alias_specs_against_value(metadata, &raw_value)?;
    let value = canonicalize_value_paths_with_alias_specs(
        &raw_value,
        &alias_specs,
        Some(shape),
        &direct_array_paths,
    )?;

    let entries = entries
        .into_iter()
        .map(|(path, trace)| {
            (
                canonicalize_layer_bookkeeping_path(&raw_value, &path, &alias_specs, shape),
                trace,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let coercible_string_paths = coercible_string_paths
        .into_iter()
        .map(|path| canonicalize_layer_bookkeeping_path(&raw_value, &path, &alias_specs, shape))
        .collect();
    let indexed_array_paths = indexed_array_paths
        .into_iter()
        .map(|path| canonicalize_layer_bookkeeping_path(&raw_value, &path, &alias_specs, shape))
        .collect();
    let indexed_array_base_lengths = indexed_array_base_lengths
        .into_iter()
        .map(|(path, length)| {
            (
                canonicalize_layer_bookkeeping_path(&raw_value, &path, &alias_specs, shape),
                length,
            )
        })
        .collect();
    let direct_array_paths = direct_array_paths
        .into_iter()
        .map(|path| canonicalize_layer_bookkeeping_path(&raw_value, &path, &alias_specs, shape))
        .collect();

    Ok(Layer {
        trace,
        value,
        entries,
        coercible_string_paths,
        indexed_array_paths,
        indexed_array_base_lengths,
        direct_array_paths,
    })
}

fn canonicalize_layer_bookkeeping_path(
    value: &Value,
    path: &str,
    alias_specs: &[AliasOverrideSpec],
    shape: &Value,
) -> String {
    let explicit_array_segments = runtime_array_segments_for_path(value, path);
    let runtime =
        try_canonicalize_runtime_path_with_explicit_arrays(value, path, &explicit_array_segments)
            .unwrap_or_else(|_| normalize_path(path));
    let (path, _) = ConfigMetadata::canonicalize_path_with_alias_specs_and_array_segments(
        &runtime,
        &explicit_array_segments,
        alias_specs,
        Some(shape),
    );
    path
}

fn runtime_array_segments_for_path(value: &Value, path: &str) -> BTreeSet<usize> {
    let mut current = value;
    let mut array_segments = BTreeSet::new();
    for (index, segment) in path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .enumerate()
    {
        match current {
            Value::Object(map) => {
                let Some(next) = map.get(segment) else {
                    break;
                };
                current = next;
            }
            Value::Array(values) => {
                array_segments.insert(index);
                let Ok(array_index) = segment.parse::<usize>() else {
                    break;
                };
                let Some(next) = values.get(array_index) else {
                    break;
                };
                current = next;
            }
            _ => break,
        }
    }
    array_segments
}
