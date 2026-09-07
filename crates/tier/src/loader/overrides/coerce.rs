use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::path::join_path;

fn coerce_all_scalars(
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
                        coerce_all_scalars(child, &next, string_coercion_paths, observed_values),
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
                    coerce_all_scalars(child, &next, string_coercion_paths, observed_values)
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

pub(in crate::loader) fn coerce_retry_scalars(
    value: &Value,
    current_path: &str,
    paths: &BTreeSet<String>,
    observed: &BTreeMap<String, Value>,
    error: &crate::loader::de::ValueDeError,
) -> Value {
    if let Some(pointer) = error.rejected_string {
        let mut retry = value.clone();
        for path in paths {
            if let Some(Value::String(raw)) = crate::path::get_value_at_path(value, path)
                && raw.as_ptr() as usize == pointer
                && let Some(converted) = retry_scalar_value(raw)
            {
                crate::path::replace_value_at_path(&mut retry, path, converted);
                break;
            }
        }
        return retry;
    }
    coerce_all_scalars(value, current_path, paths, observed)
}
