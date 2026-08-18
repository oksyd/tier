use serde_json::Value;

use crate::schema::count::{keyword_u64, len_less_than, remaining_required};

pub(super) fn child_path(parent: &str, segment: &str) -> String {
    if parent.is_empty() {
        segment.to_owned()
    } else {
        format!("{parent}.{segment}")
    }
}

pub(super) fn fixed_array_item_count(object: &serde_json::Map<String, Value>) -> usize {
    object
        .get("prefixItems")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
        .max(
            object
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
        )
}

pub(super) fn array_item_is_required(
    object: &serde_json::Map<String, Value>,
    index: usize,
) -> bool {
    keyword_u64(object, "minItems").is_some_and(|min_items| len_less_than(index, min_items))
}

pub(super) fn required_additional_array_items(
    object: &serde_json::Map<String, Value>,
    fixed_item_count: usize,
) -> usize {
    keyword_u64(object, "minItems").map_or(0, |min_items| {
        remaining_required(min_items, fixed_item_count)
    })
}
