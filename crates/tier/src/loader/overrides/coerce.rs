use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::path::join_path;

pub(in crate::loader) fn coerce_retry_scalars(
    value: &Value,
    current_path: &str,
    string_coercion_paths: &BTreeSet<String>,
    observed_values: &BTreeMap<String, Value>,
) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, child)| {
                    let next = join_path(current_path, key);
                    (
                        key.clone(),
                        coerce_retry_scalars(child, &next, string_coercion_paths, observed_values),
                    )
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .enumerate()
                .map(|(index, child)| {
                    let next = join_path(current_path, &index.to_string());
                    coerce_retry_scalars(child, &next, string_coercion_paths, observed_values)
                })
                .collect(),
        ),
        Value::String(raw)
            if string_coercion_paths.contains(current_path)
                && !observed_values
                    .get(current_path)
                    .is_some_and(Value::is_string) =>
        {
            retry_scalar_value(raw).unwrap_or_else(|| Value::String(raw.clone()))
        }
        other => other.clone(),
    }
}

fn retry_scalar_value(raw: &str) -> Option<Value> {
    let value = serde_json::from_str::<Value>(raw.trim()).ok()?;
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => Some(value),
        _ => None,
    }
}
