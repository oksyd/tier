use serde_json::{Map, Value};

use crate::schema::{
    allows_additional_array_items_for_schema, legacy_additional_items_for_schema,
    required_contains_additional_items_for_docs,
};

use super::path::{
    array_item_is_required, fixed_array_item_count, required_additional_array_items,
};

pub(super) fn fixed_item_required(
    required: bool,
    object: &Map<String, Value>,
    index: usize,
) -> bool {
    required && array_item_is_required(object, index)
}

pub(super) fn homogeneous_items_schema(object: &Map<String, Value>) -> Option<&Value> {
    object
        .get("items")
        .filter(|value| !value.is_array() && !matches!(value, Value::Bool(false)))
}

pub(super) fn legacy_additional_items_schema(object: &Map<String, Value>) -> Option<&Value> {
    legacy_additional_items_for_schema(object)
        .filter(|value| !value.is_array() && !matches!(value, Value::Bool(false)))
}

pub(super) fn contains_schema(object: &Map<String, Value>) -> Option<&Value> {
    object
        .get("contains")
        .filter(|value| !matches!(value, Value::Bool(false)))
}

pub(super) fn allows_wildcard_entries(object: &Map<String, Value>) -> bool {
    allows_additional_array_items_for_schema(object, fixed_array_item_count(object))
}

pub(super) fn additional_items_required(required: bool, object: &Map<String, Value>) -> bool {
    required && required_additional_array_items(object, fixed_array_item_count(object)) > 0
}

pub(super) fn contains_items_required(
    required: bool,
    object: &Map<String, Value>,
    root: &Value,
) -> bool {
    required && required_contains_additional_items_for_docs(object, root) > 0
}
