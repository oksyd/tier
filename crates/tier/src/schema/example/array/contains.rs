use std::collections::BTreeSet;

use serde_json::Value;

use super::super::matches::example_matches_schema;
use super::super::{build_example_value, merge_example_value};
use super::constraints::array_requires_unique_items;
use super::unique::uniquify_array_example_items;
use crate::schema::count::{keyword_u64, usize_saturating};
use crate::value::values_contain;

pub(in crate::schema::example) fn required_contains_item_count(
    object: &serde_json::Map<String, Value>,
) -> usize {
    if !object.contains_key("contains") {
        return 0;
    }

    keyword_u64(object, "minContains").map_or(1, usize_saturating)
}

pub(crate) fn required_contains_additional_items_for_docs(
    object: &serde_json::Map<String, Value>,
    root: &Value,
) -> usize {
    let Some(contains) = object.get("contains") else {
        return 0;
    };

    let required_matches = required_contains_item_count(object);
    if required_matches == 0 {
        return 0;
    }

    let mut fixed_examples = merged_fixed_example_items(object, root);
    if array_requires_unique_items(object) {
        uniquify_array_example_items(&mut fixed_examples, object, root);
    }
    let existing_matches = count_matching_example_items(
        &fixed_examples,
        contains,
        root,
        array_requires_unique_items(object),
    );
    required_matches.saturating_sub(existing_matches)
}

fn merged_fixed_example_items(object: &serde_json::Map<String, Value>, root: &Value) -> Vec<Value> {
    let mut merged = None;

    if let Some(items) = object.get("prefixItems").and_then(Value::as_array) {
        let rendered = items
            .iter()
            .map(|child| {
                build_example_value(child, root, &mut BTreeSet::new(), None).unwrap_or(Value::Null)
            })
            .collect::<Vec<_>>();
        merge_example_value(&mut merged, Value::Array(rendered));
    }

    if let Some(items) = object.get("items").and_then(Value::as_array) {
        let rendered = items
            .iter()
            .map(|child| {
                build_example_value(child, root, &mut BTreeSet::new(), None).unwrap_or(Value::Null)
            })
            .collect::<Vec<_>>();
        merge_example_value(&mut merged, Value::Array(rendered));
    }

    merged
        .and_then(|value: Value| value.as_array().cloned())
        .unwrap_or_default()
}

pub(in crate::schema::example) fn count_matching_example_items(
    values: &[Value],
    schema: &Value,
    root: &Value,
    unique_items: bool,
) -> usize {
    let mut matching = Vec::<Value>::new();
    for value in values {
        if !example_matches_schema(value, schema, root, &mut BTreeSet::new()) {
            continue;
        }
        if unique_items && values_contain(&matching, value) {
            continue;
        }
        matching.push(value.clone());
    }
    matching.len()
}
