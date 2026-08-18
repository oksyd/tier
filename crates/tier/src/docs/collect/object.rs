use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::schema::count::{keyword_u64, usize_saturating};
use crate::schema::{dynamic_object_placeholder, dynamic_object_placeholder_for_schema};

pub(super) fn required_properties(object: &Map<String, Value>) -> BTreeSet<String> {
    object
        .get("required")
        .and_then(Value::as_array)
        .map(|required| {
            required
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn min_properties(object: &Map<String, Value>) -> usize {
    keyword_u64(object, "minProperties").map_or(0, usize_saturating)
}

pub(super) fn max_properties(object: &Map<String, Value>) -> Option<usize> {
    keyword_u64(object, "maxProperties").map(usize_saturating)
}

pub(super) fn all_known_properties_required(
    object: &Map<String, Value>,
    properties: &Map<String, Value>,
    min_properties: usize,
) -> bool {
    object
        .get("additionalProperties")
        .and_then(Value::as_bool)
        .is_some_and(|allowed| !allowed)
        && !properties.is_empty()
        && min_properties >= properties.len()
}

pub(super) fn allows_optional_properties(
    max_properties: Option<usize>,
    required_property_count: usize,
) -> bool {
    max_properties.is_none_or(|max_properties| max_properties > required_property_count)
}

pub(super) fn required_dynamic_properties(
    required: bool,
    min_properties: usize,
    known_property_count: usize,
) -> bool {
    required && min_properties > known_property_count
}

pub(super) fn allows_dynamic_properties(
    max_properties: Option<usize>,
    required_fixed_property_count: usize,
) -> bool {
    max_properties.is_none_or(|max_properties| max_properties > required_fixed_property_count)
}

pub(super) fn dynamic_property_segment(
    object: &Map<String, Value>,
    root: &Value,
    reserved_keys: &BTreeSet<String>,
) -> Option<String> {
    let dynamic_keys_allowed = object.get("propertyNames").is_none_or(|_| {
        dynamic_object_placeholder_for_schema(object, root, reserved_keys).is_some()
    });
    if !dynamic_keys_allowed {
        return None;
    }

    let placeholder = dynamic_object_placeholder(reserved_keys);
    Some(if placeholder == "{item}" {
        "*".to_owned()
    } else {
        placeholder
    })
}

pub(super) fn additional_properties_schema<'a>(
    object: &'a Map<String, Value>,
    required_dynamic_properties: bool,
    implicit_additional: &'a Value,
) -> Option<&'a Value> {
    object
        .get("additionalProperties")
        .filter(|value| !matches!(value, Value::Bool(false)))
        .or_else(|| {
            if !object.contains_key("additionalProperties") && required_dynamic_properties {
                Some(implicit_additional)
            } else {
                None
            }
        })
}
