use std::collections::BTreeSet;

use serde_json::Value;

use super::{build_example_value, merge_example_value, value_satisfies_schema};

pub(super) fn merge_all_of_examples(
    parent_schema: &Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    reserved_keys: &BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let Some(values) = object.get("allOf").and_then(Value::as_array) else {
        return;
    };

    let mut all_of_merged = None;
    for child in values {
        let Some(example) = build_example_value(child, root, visited_refs, Some(reserved_keys))
        else {
            continue;
        };
        merge_example_value(&mut all_of_merged, example);
    }

    if let Some(example) = all_of_merged
        && example_can_seed_parent_schema(&example, parent_schema, root)
    {
        merge_example_value(merged, example);
    }
}

pub(super) fn merge_first_non_null_branch_example(
    keyword: &str,
    parent_schema: &Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    reserved_keys: &BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let Some(values) = object.get(keyword).and_then(Value::as_array) else {
        return;
    };

    for candidate in values {
        if schema_is_null(candidate) {
            continue;
        }
        if let Some(example) =
            build_example_value(candidate, root, visited_refs, Some(reserved_keys))
        {
            if !example_can_seed_parent_schema(&example, parent_schema, root) {
                continue;
            }
            merge_example_value(merged, example);
            break;
        }
    }
}

fn schema_is_null(schema: &Value) -> bool {
    if schema.get("const").is_some_and(Value::is_null) {
        return true;
    }

    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return !values.is_empty() && values.iter().all(Value::is_null);
    }

    match schema.get("type") {
        Some(Value::String(ty)) => ty == "null",
        Some(Value::Array(types)) => {
            !types.is_empty() && types.iter().all(|ty| ty.as_str() == Some("null"))
        }
        _ => false,
    }
}

fn example_can_seed_parent_schema(example: &Value, parent_schema: &Value, root: &Value) -> bool {
    match example {
        // Object and array examples are often assembled from multiple schema locations.
        // Keep them composable even when the current partial value does not yet satisfy
        // parent-level required/minItems constraints.
        Value::Object(_) | Value::Array(_) => true,
        _ => value_satisfies_schema(example, parent_schema, root),
    }
}
