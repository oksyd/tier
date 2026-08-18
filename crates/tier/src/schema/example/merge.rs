use serde_json::Value;

pub(super) fn merge_example_value(target: &mut Option<Value>, overlay: Value) {
    match target {
        Some(current) => merge_example_branch(current, overlay),
        None => *target = Some(overlay),
    }
}

fn merge_example_branch(target: &mut Value, overlay: Value) {
    match (target, overlay) {
        (Value::Object(existing), Value::Object(overlay_map)) => {
            merge_example_object(existing, overlay_map);
        }
        (Value::Array(existing), Value::Array(overlay_items)) => {
            merge_example_array(existing, overlay_items);
        }
        (existing, other) if existing.is_null() && !other.is_null() => *existing = other,
        (existing, other) if !other.is_null() => *existing = other,
        _ => {}
    }
}

fn merge_example_object(
    target: &mut serde_json::Map<String, Value>,
    overlay: serde_json::Map<String, Value>,
) {
    for (key, value) in overlay {
        if let Some(existing) = target.get_mut(&key) {
            merge_example_branch(existing, value);
        } else {
            target.insert(key, value);
        }
    }
}

fn merge_example_array(target: &mut Vec<Value>, overlay: Vec<Value>) {
    for (index, value) in overlay.into_iter().enumerate() {
        if let Some(existing) = target.get_mut(index) {
            merge_example_branch(existing, value);
        } else {
            target.push(value);
        }
    }
}
