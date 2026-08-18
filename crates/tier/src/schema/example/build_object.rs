use std::collections::BTreeSet;

use serde_json::Value;

use super::object::{
    dynamic_object_placeholders_for_schema, pattern_property_placeholder_for_schema,
    trim_object_example_properties,
};
use super::{build_example_value, merge_example_value};
use crate::schema::count::{
    available_slots as count_available_slots, keyword_u64, remaining_required,
};

pub(super) fn merge_object_examples(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    reserved_keys: &BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    merge_fixed_property_examples(object, root, visited_refs, merged);
    merge_pattern_property_examples(object, root, visited_refs, reserved_keys, merged);
}

pub(super) fn merge_additional_property_examples(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    reserved_keys: &BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let existing_object_len = merged
        .as_ref()
        .and_then(Value::as_object)
        .map_or(0, serde_json::Map::len);
    let required_dynamic = keyword_u64(object, "minProperties").map_or(0, |min_properties| {
        remaining_required(min_properties, existing_object_len)
    });
    let implicit_additional = Value::Bool(true);
    let additional_properties = object.get("additionalProperties").or({
        if required_dynamic > 0 {
            Some(&implicit_additional)
        } else {
            None
        }
    });
    if let Some(additional) = additional_properties
        && let Some(example) = build_example_value(additional, root, visited_refs, None)
    {
        let available_slots =
            count_available_slots(keyword_u64(object, "maxProperties"), existing_object_len);
        let additional_entries = if object.contains_key("additionalProperties") {
            required_dynamic.max(1).min(available_slots)
        } else {
            required_dynamic.min(available_slots)
        };
        if additional_entries > 0 {
            let placeholders = dynamic_object_placeholders_for_schema(
                object,
                root,
                reserved_keys,
                additional_entries,
            );
            let rendered = placeholders
                .into_iter()
                .map(|placeholder| (placeholder, example.clone()))
                .collect::<serde_json::Map<_, _>>();
            merge_example_value(merged, Value::Object(rendered));
        }
    }
}

fn merge_fixed_property_examples(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        return;
    };

    let mut rendered = serde_json::Map::new();
    for (key, child) in properties {
        if let Some(example) = build_example_value(child, root, visited_refs, None) {
            rendered.insert(key.clone(), example);
        }
    }
    trim_object_example_properties(&mut rendered, object);
    merge_example_value(merged, Value::Object(rendered));
}

fn merge_pattern_property_examples(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    reserved_keys: &BTreeSet<String>,
    merged: &mut Option<Value>,
) {
    let Some(pattern_properties) = object.get("patternProperties").and_then(Value::as_object)
    else {
        return;
    };
    let existing_len = merged
        .as_ref()
        .and_then(Value::as_object)
        .map_or(0, serde_json::Map::len);
    let required_dynamic = keyword_u64(object, "minProperties").map_or(0, |min_properties| {
        remaining_required(min_properties, existing_len)
    });
    let available_slots = count_available_slots(keyword_u64(object, "maxProperties"), existing_len);
    if available_slots == 0 {
        return;
    }

    let mut taken = reserved_keys.clone();
    if let Some(existing) = merged.as_ref().and_then(Value::as_object) {
        taken.extend(existing.keys().cloned());
    }

    let mut rendered = serde_json::Map::new();
    let target_entries = available_slots.min(required_dynamic.max(pattern_properties.len()));
    while rendered.len() < target_entries {
        let mut made_progress = false;
        for (pattern, child) in pattern_properties {
            if rendered.len() >= target_entries {
                break;
            }
            if let Some(key) =
                pattern_property_placeholder_for_schema(pattern, object, root, &taken)
                && let Some(example) = build_example_value(child, root, visited_refs, None)
            {
                taken.insert(key.clone());
                rendered.insert(key, example);
                made_progress = true;
            }
        }

        if !made_progress {
            break;
        }
    }
    if !rendered.is_empty() {
        merge_example_value(merged, Value::Object(rendered));
    }
}
