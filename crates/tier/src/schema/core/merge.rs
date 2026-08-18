use serde_json::Value;

pub(super) fn merge_schema_keyword(
    target: &mut serde_json::Map<String, Value>,
    key: &str,
    overlay: &Value,
) {
    match key {
        "required" => merge_unique_array_keyword(target, key, overlay),
        "prefixItems" | "items" if overlay.is_array() => {
            merge_indexed_array_keyword(target, key, overlay);
        }
        "allOf" | "anyOf" | "oneOf" => merge_append_array_keyword(target, key, overlay),
        _ => merge_object_or_replace_keyword(target, key, overlay),
    }
}

fn merge_unique_array_keyword(
    target: &mut serde_json::Map<String, Value>,
    key: &str,
    overlay: &Value,
) {
    let mut merged = target
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(values) = overlay.as_array() else {
        target.insert(key.to_owned(), overlay.clone());
        return;
    };

    for value in values {
        if !merged.contains(value) {
            merged.push(value.clone());
        }
    }
    target.insert(key.to_owned(), Value::Array(merged));
}

fn merge_indexed_array_keyword(
    target: &mut serde_json::Map<String, Value>,
    key: &str,
    overlay: &Value,
) {
    let mut merged = target
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(values) = overlay.as_array() else {
        target.insert(key.to_owned(), overlay.clone());
        return;
    };

    merge_schema_arrays(&mut merged, values);
    target.insert(key.to_owned(), Value::Array(merged));
}

fn merge_append_array_keyword(
    target: &mut serde_json::Map<String, Value>,
    key: &str,
    overlay: &Value,
) {
    let mut merged = target
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(values) = overlay.as_array() else {
        target.insert(key.to_owned(), overlay.clone());
        return;
    };

    merged.extend(values.iter().cloned());
    target.insert(key.to_owned(), Value::Array(merged));
}

fn merge_object_or_replace_keyword(
    target: &mut serde_json::Map<String, Value>,
    key: &str,
    overlay: &Value,
) {
    match (target.get_mut(key), overlay) {
        (Some(Value::Object(existing)), Value::Object(overlay_map)) => {
            merge_schema_objects(existing, overlay_map);
        }
        _ => {
            target.insert(key.to_owned(), overlay.clone());
        }
    }
}

fn merge_schema_objects(
    target: &mut serde_json::Map<String, Value>,
    overlay: &serde_json::Map<String, Value>,
) {
    for (key, value) in overlay {
        merge_schema_keyword(target, key, value);
    }
}

fn merge_schema_arrays(target: &mut Vec<Value>, overlay: &[Value]) {
    for (index, value) in overlay.iter().enumerate() {
        if value.is_null() {
            continue;
        }
        if let Some(existing) = target.get_mut(index) {
            merge_schema_value(existing, value);
        } else {
            target.push(value.clone());
        }
    }
}

fn merge_schema_value(target: &mut Value, overlay: &Value) {
    match (target, overlay) {
        (Value::Object(existing), Value::Object(overlay_map)) => {
            merge_schema_objects(existing, overlay_map);
        }
        (Value::Array(existing), Value::Array(overlay_items)) => {
            merge_schema_arrays(existing, overlay_items);
        }
        (target, overlay) => *target = overlay.clone(),
    }
}
