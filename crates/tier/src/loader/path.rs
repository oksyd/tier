use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::ConfigError;
use crate::path::{
    checked_array_len_for_index, is_array_index_segment, join_path, normalize_path,
    path_is_at_or_below,
};

pub(super) use crate::path::direct_child_array_index;

pub(super) fn ensure_path_safe_keys(value: &Value, current_path: &str) -> Result<(), ConfigError> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                validate_path_key(current_path, key)?;
                let next = join_path(current_path, key);
                ensure_path_safe_keys(child, &next)?;
            }
            Ok(())
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                let next = join_path(current_path, &index.to_string());
                ensure_path_safe_keys(child, &next)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_path_key(current_path: &str, key: &str) -> Result<(), ConfigError> {
    let message = invalid_path_key_message(key);
    if let Some(message) = message {
        Err(ConfigError::InvalidPathKey {
            path: current_path.to_owned(),
            key: key.to_owned(),
            message,
        })
    } else {
        Ok(())
    }
}

pub(super) fn invalid_path_key_message(key: &str) -> Option<String> {
    if key.is_empty() {
        Some("empty object keys are not supported".to_owned())
    } else if key == "*" {
        Some("`*` is reserved for wildcard metadata paths".to_owned())
    } else if key.contains('.') {
        Some("`.` is reserved as the configuration path separator".to_owned())
    } else if key.contains('[') || key.contains(']') {
        Some("`[` and `]` are reserved for external array path syntax".to_owned())
    } else {
        None
    }
}

pub(super) fn invalid_concrete_path_segment(path: &str) -> Option<(&str, String)> {
    path.split('.')
        .find_map(|segment| invalid_path_key_message(segment).map(|message| (segment, message)))
}

pub(crate) fn indexed_array_container_paths(segments: &[&str]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for index in 0..segments.len() {
        if is_array_index_segment(segments[index]) && index > 0 {
            paths.insert(segments[..index].join("."));
        }
    }
    paths
}

pub(crate) fn record_indexed_array_state(
    current_array_lengths: &mut BTreeMap<String, usize>,
    indexed_array_base_lengths: &mut BTreeMap<String, usize>,
    path: &str,
    segments: &[&str],
) {
    for container_path in indexed_array_container_paths(segments) {
        let Some(index) = direct_child_array_index(&container_path, path) else {
            continue;
        };
        let Some(current_length) = current_array_lengths.get_mut(&container_path) else {
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

pub(crate) fn record_direct_array_state(
    current_array_lengths: &mut BTreeMap<String, usize>,
    indexed_array_base_lengths: &mut BTreeMap<String, usize>,
    path: &str,
    value: &Value,
) {
    clear_array_state(current_array_lengths, path);
    clear_array_state(indexed_array_base_lengths, path);
    collect_array_lengths(value, path, current_array_lengths);
}

fn clear_array_state<T>(state: &mut BTreeMap<String, T>, path: &str) {
    state.retain(|candidate, _| !path_is_at_or_below(candidate, path));
}

fn collect_array_lengths(value: &Value, path: &str, lengths: &mut BTreeMap<String, usize>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = join_path(path, key);
                collect_array_lengths(child, &next, lengths);
            }
        }
        Value::Array(values) => {
            lengths.insert(path.to_owned(), values.len());
            for (index, child) in values.iter().enumerate() {
                let next = join_path(path, &index.to_string());
                collect_array_lengths(child, &next, lengths);
            }
        }
        _ => {}
    }
}

pub(crate) fn normalize_external_path(path: &str) -> String {
    try_normalize_external_path(path).unwrap_or_else(|_| normalize_path(path))
}

pub(crate) fn try_normalize_external_path(path: &str) -> Result<String, String> {
    crate::path::normalize_external_path(path)
}

pub(crate) fn try_normalize_external_path_with_explicit_arrays(
    path: &str,
) -> Result<(String, BTreeSet<usize>), String> {
    crate::path::normalize_external_path_with_explicit_arrays(path)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::clear_array_state;

    #[test]
    fn clear_array_state_removes_replaced_path_and_descendants_only() {
        let mut state = BTreeMap::from([
            ("users".to_owned(), 2),
            ("users.0.roles".to_owned(), 1),
            ("users_profile".to_owned(), 1),
        ]);

        clear_array_state(&mut state, "users");

        assert_eq!(state, BTreeMap::from([("users_profile".to_owned(), 1)]));
    }

    #[test]
    fn clear_array_state_removes_everything_for_root_replacement() {
        let mut state = BTreeMap::from([
            ("".to_owned(), 1),
            ("0.roles".to_owned(), 2),
            ("service.users".to_owned(), 3),
        ]);

        clear_array_state(&mut state, "");

        assert!(state.is_empty());
    }
}
