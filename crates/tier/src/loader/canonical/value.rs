use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::metadata::AliasOverrideSpec;
use crate::path::join_path;
use crate::{ConfigError, ConfigMetadata};

use super::super::de::insert_path_with_shape_and_explicit_arrays;
use super::super::merge::ensure_root_object;
use super::super::path::ensure_path_safe_keys;

pub(in crate::loader) fn canonicalize_value_paths(
    value: &Value,
    metadata: &ConfigMetadata,
) -> Result<Value, ConfigError> {
    ensure_root_object(value)?;
    ensure_path_safe_keys(value, "")?;

    let aliases = metadata.alias_override_specs()?;
    canonicalize_value_paths_with_alias_specs(value, &aliases, Some(value), &BTreeSet::new())
}

pub(in crate::loader) fn canonicalize_value_paths_with_alias_specs(
    value: &Value,
    aliases: &[AliasOverrideSpec],
    shape: Option<&Value>,
    direct_array_paths: &BTreeSet<String>,
) -> Result<Value, ConfigError> {
    ensure_root_object(value)?;
    ensure_path_safe_keys(value, "")?;
    if aliases.is_empty() {
        return Ok(value.clone());
    }

    let mut canonical = Value::Object(Map::new());
    let mut nodes = Vec::new();
    collect_value_nodes(
        value,
        "",
        0,
        &BTreeSet::new(),
        direct_array_paths,
        &mut nodes,
    );
    let mut seen = BTreeMap::<String, String>::new();

    for node in nodes {
        let (canonical_path, array_segments) =
            ConfigMetadata::canonicalize_path_with_alias_specs_and_array_segments(
                &node.path,
                &node.array_segments,
                aliases,
                shape,
            );
        if let Some(first_path) = seen.get(&canonical_path)
            && first_path != &node.path
        {
            return Err(ConfigError::PathConflict {
                first_path: first_path.clone(),
                second_path: node.path,
                canonical_path,
            });
        }
        seen.insert(canonical_path.clone(), node.path);
        let segments = canonical_path.split('.').collect::<Vec<_>>();
        let insertion_shape = shape.unwrap_or(value);
        insert_path_with_shape_and_explicit_arrays(
            &mut canonical,
            Some(insertion_shape),
            &segments,
            &array_segments,
            node.value,
        )
        .map_err(|message| ConfigError::InvalidArg {
            arg: canonical_path.clone(),
            message,
        })?;
    }

    Ok(canonical)
}

struct ValueNode {
    path: String,
    value: Value,
    array_segments: BTreeSet<usize>,
}

fn collect_value_nodes(
    value: &Value,
    current: &str,
    depth: usize,
    array_segments: &BTreeSet<usize>,
    direct_array_paths: &BTreeSet<String>,
    nodes: &mut Vec<ValueNode>,
) {
    match value {
        Value::Array(_) if !current.is_empty() && direct_array_paths.contains(current) => {
            nodes.push(ValueNode {
                path: current.to_owned(),
                value: value.clone(),
                array_segments: array_segments.clone(),
            });
        }
        Value::Object(map) if map.is_empty() && !current.is_empty() => {
            nodes.push(ValueNode {
                path: current.to_owned(),
                value: Value::Object(Map::new()),
                array_segments: array_segments.clone(),
            });
        }
        Value::Object(map) => {
            for (key, child) in map {
                let next = join_path(current, key);
                collect_value_nodes(
                    child,
                    &next,
                    depth + 1,
                    array_segments,
                    direct_array_paths,
                    nodes,
                );
            }
        }
        Value::Array(values) if values.is_empty() && !current.is_empty() => {
            nodes.push(ValueNode {
                path: current.to_owned(),
                value: Value::Array(Vec::new()),
                array_segments: array_segments.clone(),
            });
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                let next = join_path(current, &index.to_string());
                let mut child_array_segments = array_segments.clone();
                child_array_segments.insert(depth);
                collect_value_nodes(
                    child,
                    &next,
                    depth + 1,
                    &child_array_segments,
                    direct_array_paths,
                    nodes,
                );
            }
        }
        _ if !current.is_empty() => nodes.push(ValueNode {
            path: current.to_owned(),
            value: value.clone(),
            array_segments: array_segments.clone(),
        }),
        _ => {}
    }
}
