use serde_json::Value;

use crate::schema::count::{available_slots, keyword_u64, len_less_than, remaining_required};

pub(crate) fn allows_additional_array_items_for_schema(
    object: &serde_json::Map<String, Value>,
    fixed_item_count: usize,
) -> bool {
    if additional_array_items_forbidden(object) {
        return false;
    }

    keyword_u64(object, "maxItems")
        .is_none_or(|max_items| len_less_than(fixed_item_count, max_items))
}

fn additional_array_items_forbidden(object: &serde_json::Map<String, Value>) -> bool {
    object
        .get("items")
        .is_some_and(|value| matches!(value, Value::Bool(false)))
        || legacy_additional_items_for_schema(object)
            .is_some_and(|value| matches!(value, Value::Bool(false)))
}

pub(crate) fn legacy_additional_items_for_schema(
    object: &serde_json::Map<String, Value>,
) -> Option<&Value> {
    object.get("items").filter(|value| value.is_array())?;
    object.get("additionalItems")
}

pub(in crate::schema::example) fn available_additional_array_slots(
    object: &serde_json::Map<String, Value>,
    existing_len: usize,
) -> usize {
    if !allows_additional_array_items_for_schema(object, existing_len) {
        return 0;
    }

    available_slots(keyword_u64(object, "maxItems"), existing_len)
}

pub(in crate::schema::example) fn array_requires_unique_items(
    object: &serde_json::Map<String, Value>,
) -> bool {
    object
        .get("uniqueItems")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(in crate::schema::example) fn additional_example_item_count(
    object: &serde_json::Map<String, Value>,
    fixed_item_count: usize,
) -> usize {
    if !allows_additional_array_items_for_schema(object, fixed_item_count) {
        return 0;
    }

    let required_additional = keyword_u64(object, "minItems").map_or(0, |min_items| {
        remaining_required(min_items, fixed_item_count)
    });
    required_additional.max(1)
}
