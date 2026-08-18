use std::collections::BTreeSet;

use serde_json::Value;

use super::parse_array_index_segment;

use super::{join_path, path_starts_with_pattern};
use crate::value::values_equal;

pub(crate) fn redact_value(value: &Value, path: &str, secret_paths: &BTreeSet<String>) -> Value {
    if secret_paths
        .iter()
        .any(|secret| path_starts_with_pattern(path, secret))
    {
        return Value::String("***redacted***".to_owned());
    }

    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let next = join_path(path, key);
                    (key.clone(), redact_value(value, &next, secret_paths))
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let next = join_path(path, &index.to_string());
                    redact_value(value, &next, secret_paths)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

pub(crate) fn collect_paths(value: &Value, current: &str, paths: &mut Vec<String>) {
    if !current.is_empty() {
        paths.push(current.to_owned());
    }

    if let Value::Object(map) = value {
        for (key, child) in map {
            let next = join_path(current, key);
            collect_paths(child, &next, paths);
        }
    } else if let Value::Array(values) = value {
        for (index, child) in values.iter().enumerate() {
            let next = join_path(current, &index.to_string());
            collect_paths(child, &next, paths);
        }
    }
}

pub(crate) fn get_value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(value);
    }

    let mut current = value;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            Value::Array(values) => {
                let index = parse_array_index_segment(segment).ok()?;
                current = values.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

pub(crate) fn collect_diff_paths(
    before: &Value,
    after: &Value,
    current: &str,
    paths: &mut Vec<String>,
) {
    if values_equal(before, after) {
        return;
    }

    if !current.is_empty() {
        paths.push(current.to_owned());
    }

    if let (Value::Object(before_map), Value::Object(after_map)) = (before, after) {
        let keys = before_map
            .keys()
            .chain(after_map.keys())
            .collect::<BTreeSet<_>>();
        for key in keys {
            let before_child = before_map.get(key).unwrap_or(&Value::Null);
            let after_child = after_map.get(key).unwrap_or(&Value::Null);
            let next = join_path(current, key);
            collect_diff_paths(before_child, after_child, &next, paths);
        }
    } else if let (Value::Array(before_values), Value::Array(after_values)) = (before, after) {
        let len = before_values.len().max(after_values.len());
        for index in 0..len {
            let before_child = before_values.get(index).unwrap_or(&Value::Null);
            let after_child = after_values.get(index).unwrap_or(&Value::Null);
            let next = join_path(current, &index.to_string());
            collect_diff_paths(before_child, after_child, &next, paths);
        }
    } else {
        if matches!(before, Value::Object(_) | Value::Array(_)) {
            collect_paths(before, current, paths);
        }
        if matches!(after, Value::Object(_) | Value::Array(_)) {
            collect_paths(after, current, paths);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::collect_diff_paths;

    #[test]
    fn diff_paths_treat_numeric_values_semantically() {
        let before = json!({
            "port": 8080,
            "nested": [{ "weight": 1 }]
        });
        let after = json!({
            "port": 8080.0,
            "nested": [{ "weight": 1.0 }]
        });
        let mut paths = Vec::new();

        collect_diff_paths(&before, &after, "", &mut paths);

        assert!(paths.is_empty());
    }

    #[test]
    fn diff_paths_still_report_real_numeric_changes() {
        let before = json!({ "port": 8080 });
        let after = json!({ "port": 8081 });
        let mut paths = Vec::new();

        collect_diff_paths(&before, &after, "", &mut paths);

        assert_eq!(paths, vec!["port".to_owned()]);
    }
}
