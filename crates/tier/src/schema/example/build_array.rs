use std::collections::BTreeSet;

use serde_json::Value;

use super::array::{
    additional_example_item_count, array_requires_unique_items, available_additional_array_slots,
    build_repeated_example_values, count_matching_example_items, required_contains_item_count,
};
use super::{build_example_value, legacy_additional_items_for_schema, merge_example_value};

pub(super) fn merge_array_examples(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    merge_fixed_array_examples("prefixItems", object, root, visited_refs, merged);
    merge_fixed_array_examples("items", object, root, visited_refs, merged);
    merge_homogeneous_array_examples(
        object.get("items").filter(|value| !value.is_array()),
        object,
        root,
        visited_refs,
        merged,
    );
    merge_homogeneous_array_examples(
        legacy_additional_items_for_schema(object).filter(|value| !value.is_array()),
        object,
        root,
        visited_refs,
        merged,
    );
}

fn merge_fixed_array_examples(
    keyword: &str,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let Some(items) = object.get(keyword).and_then(Value::as_array) else {
        return;
    };

    let rendered = items
        .iter()
        .map(|child| build_example_value(child, root, visited_refs, None))
        .take_while(Option::is_some)
        .flatten()
        .collect::<Vec<_>>();
    if !rendered.is_empty() {
        merge_example_value(merged, Value::Array(rendered));
    }
}

fn merge_homogeneous_array_examples(
    item_schema: Option<&Value>,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let Some(item_schema) = item_schema else {
        return;
    };

    let existing_len = merged
        .as_ref()
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if existing_len < fixed_item_count(object) {
        return;
    }
    let additional_items = additional_example_item_count(object, existing_len);
    if additional_items == 0 {
        return;
    }

    match build_example_value(item_schema, root, visited_refs, None) {
        Some(example) => match merged {
            Some(Value::Array(existing)) => {
                existing.extend(std::iter::repeat_n(example, additional_items));
            }
            _ => {
                merge_example_value(
                    merged,
                    Value::Array(std::iter::repeat_n(example, additional_items).collect()),
                );
            }
        },
        None => {
            if merged.is_none() {
                merge_example_value(merged, Value::Array(Vec::new()));
            }
        }
    }
}

fn fixed_item_count(object: &serde_json::Map<String, Value>) -> usize {
    let prefix_items = object
        .get("prefixItems")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let tuple_items = object
        .get("items")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    prefix_items.max(tuple_items)
}

pub(super) fn merge_contains_array_examples(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let Some(contains) = object.get("contains") else {
        return;
    };
    let required_matches = required_contains_item_count(object);
    if required_matches == 0 {
        return;
    }

    let unique_items = array_requires_unique_items(object);
    let existing_matches = merged
        .as_ref()
        .and_then(Value::as_array)
        .map_or(0, |values| {
            count_matching_example_items(values, contains, root, unique_items)
        });
    let missing = required_matches.saturating_sub(existing_matches);
    let available_slots = available_additional_array_slots(
        object,
        merged
            .as_ref()
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
    );
    let additional_items = missing.min(available_slots);
    if additional_items == 0 {
        return;
    }

    match build_example_value(contains, root, visited_refs, None) {
        Some(example) => match merged {
            Some(Value::Array(existing)) => {
                let additions = build_repeated_example_values(
                    example,
                    contains,
                    root,
                    additional_items,
                    unique_items,
                    existing,
                );
                existing.extend(additions);
            }
            _ => {
                let additions = build_repeated_example_values(
                    example,
                    contains,
                    root,
                    additional_items,
                    unique_items,
                    &[],
                );
                merge_example_value(merged, Value::Array(additions));
            }
        },
        None => {
            if merged.is_none() {
                merge_example_value(merged, Value::Array(Vec::new()));
            }
        }
    }
}
