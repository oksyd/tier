use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ConfigError;
use crate::path::{get_value_at_path, join_path};

use super::path::direct_child_array_index;
use super::{Layer, SourceKind, SourceTrace};

pub(super) fn validate_indexed_array_paths(base: &Value, layer: &Layer) -> Result<(), ConfigError> {
    for path in &layer.indexed_array_paths {
        let base_len = if let Some(base_len) = layer.indexed_array_base_lengths.get(path) {
            *base_len
        } else if layer.direct_array_paths.contains(path) {
            continue;
        } else {
            match get_value_at_path(base, path) {
                Some(Value::Array(values)) => values.len(),
                _ => 0,
            }
        };

        let mut explicit_indices = layer
            .entries
            .keys()
            .filter_map(|entry_path| direct_child_array_index(path, entry_path))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if explicit_indices.is_empty() {
            continue;
        }
        explicit_indices.retain(|index| *index >= base_len);
        if explicit_indices.is_empty() {
            continue;
        }

        for (offset, index) in explicit_indices.iter().enumerate() {
            let expected = base_len + offset;
            if *index != expected {
                return Err(sparse_indexed_array_error(layer, path, *index, expected));
            }
        }
    }

    Ok(())
}

fn sparse_indexed_array_error(
    layer: &Layer,
    container_path: &str,
    offending_index: usize,
    expected_index: usize,
) -> ConfigError {
    let offending_path = join_path(container_path, &offending_index.to_string());
    let source = layer
        .entries
        .iter()
        .filter(|(entry_path, _)| {
            direct_child_array_index(container_path, entry_path) == Some(offending_index)
        })
        .max_by_key(|(entry_path, trace)| {
            (
                !is_generic_layer_trace(trace),
                entry_path.split('.').count(),
                entry_path.len(),
            )
        })
        .or_else(|| {
            layer
                .entries
                .iter()
                .find(|(entry_path, _)| *entry_path == &offending_path)
        });
    let message = format!(
        "sparse array override at `{container_path}`: index {offending_index} requires index {expected_index} to be set first"
    );

    match source {
        Some((_, trace)) => match trace.kind {
            SourceKind::Environment => ConfigError::InvalidEnv {
                name: trace.name.clone(),
                path: offending_path,
                message,
            },
            SourceKind::Arguments => ConfigError::InvalidArg {
                arg: trace.name.clone(),
                message,
            },
            _ => ConfigError::MetadataInvalid {
                path: offending_path,
                message,
            },
        },
        None => ConfigError::MetadataInvalid {
            path: offending_path,
            message,
        },
    }
}

fn is_generic_layer_trace(trace: &SourceTrace) -> bool {
    matches!(
        (trace.kind, trace.name.as_str()),
        (SourceKind::Arguments, "arguments") | (SourceKind::Environment, "environment")
    )
}
