use std::collections::BTreeSet;

use serde_json::Value;

use crate::path::{checked_array_len_for_index, join_path};
use crate::{ConfigError, ConfigMetadata, MergeStrategy};

use super::Layer;

pub(crate) fn ensure_root_object(value: &Value) -> Result<(), ConfigError> {
    if matches!(value, Value::Object(_)) {
        Ok(())
    } else {
        Err(ConfigError::RootMustBeObject {
            actual: value_kind(value),
        })
    }
}

pub(crate) fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub(super) fn merged_shape_from_layers(
    defaults: &Value,
    layers: &[Layer],
    metadata: &ConfigMetadata,
) -> Result<Value, ConfigError> {
    let mut shape = defaults.clone();
    ensure_root_object(&shape)?;
    for layer in layers {
        merge_values(
            &mut shape,
            layer.value.clone(),
            "",
            metadata,
            &layer.indexed_array_paths,
            &layer.direct_array_paths,
        )?;
    }
    Ok(shape)
}

pub(super) fn merge_values(
    target: &mut Value,
    overlay: Value,
    current_path: &str,
    metadata: &ConfigMetadata,
    indexed_array_paths: &BTreeSet<String>,
    direct_array_paths: &BTreeSet<String>,
) -> Result<(), ConfigError> {
    let strategy = metadata
        .merge_strategy_for(current_path)
        .unwrap_or(MergeStrategy::Merge);
    let indexed_array_patch =
        indexed_array_paths.contains(current_path) && !direct_array_paths.contains(current_path);

    match (target, overlay, strategy) {
        (Value::Array(target), Value::Array(overlay), _)
            if indexed_array_patch && !current_path.is_empty() =>
        {
            merge_indexed_array_patch(
                target,
                overlay,
                current_path,
                metadata,
                indexed_array_paths,
                direct_array_paths,
            )?;
        }
        (target, overlay, MergeStrategy::Replace) if !current_path.is_empty() => *target = overlay,
        (target, overlay, MergeStrategy::Append) => match (target, overlay) {
            (Value::Array(target), Value::Array(mut overlay)) => target.append(&mut overlay),
            (Value::Object(target), Value::Object(overlay)) => {
                for (key, value) in overlay {
                    let path = join_path(current_path, &key);
                    match target.get_mut(&key) {
                        Some(existing) => merge_values(
                            existing,
                            value,
                            &path,
                            metadata,
                            indexed_array_paths,
                            direct_array_paths,
                        )?,
                        None => {
                            target.insert(key, value);
                        }
                    }
                }
            }
            (target, overlay) => *target = overlay,
        },
        (target, overlay, MergeStrategy::Merge | MergeStrategy::Replace) => match (target, overlay)
        {
            (Value::Object(target), Value::Object(overlay)) => {
                for (key, value) in overlay {
                    let path = join_path(current_path, &key);
                    match target.get_mut(&key) {
                        Some(existing) => merge_values(
                            existing,
                            value,
                            &path,
                            metadata,
                            indexed_array_paths,
                            direct_array_paths,
                        )?,
                        None => {
                            target.insert(key, value);
                        }
                    }
                }
            }
            (target, overlay) => *target = overlay,
        },
    }

    Ok(())
}

fn merge_indexed_array_patch(
    target: &mut Vec<Value>,
    overlay: Vec<Value>,
    current_path: &str,
    metadata: &ConfigMetadata,
    indexed_array_paths: &BTreeSet<String>,
    direct_array_paths: &BTreeSet<String>,
) -> Result<(), ConfigError> {
    for (index, value) in overlay.into_iter().enumerate() {
        if value.is_null() {
            continue;
        }

        let path = join_path(current_path, &index.to_string());
        if target.len() <= index {
            target.resize(
                checked_array_len_for_index(index).map_err(|message| {
                    ConfigError::MetadataInvalid {
                        path: path.clone(),
                        message,
                    }
                })?,
                Value::Null,
            );
        }

        if target[index].is_null() {
            target[index] = value;
            continue;
        }

        merge_values(
            &mut target[index],
            value,
            &path,
            metadata,
            indexed_array_paths,
            direct_array_paths,
        )?;
    }

    Ok(())
}
