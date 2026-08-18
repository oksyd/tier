use std::collections::BTreeSet;

use serde_json::Value;

use super::refs::inlined_schema_ref;

pub(crate) fn merged_object_level_property_names(
    schema: &Value,
    root: &Value,
    inherited: Option<&BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut reserved = BTreeSet::new();
    collect_object_level_property_names(schema, root, &mut reserved, &mut BTreeSet::new());
    if let Some(inherited) = inherited {
        reserved.extend(inherited.iter().cloned());
    }
    reserved
}

fn collect_object_level_property_names(
    schema: &Value,
    root: &Value,
    reserved: &mut BTreeSet<String>,
    visited_refs: &mut BTreeSet<String>,
) {
    let Some(object) = schema.as_object() else {
        return;
    };

    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        if visited_refs.insert(reference.to_owned()) {
            if let Some(inlined) = inlined_schema_ref(schema, root) {
                collect_object_level_property_names(&inlined, root, reserved, visited_refs);
            }
            visited_refs.remove(reference);
        }
        return;
    }

    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        reserved.extend(properties.keys().cloned());
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            for child in children {
                collect_object_level_property_names(child, root, reserved, visited_refs);
            }
        }
    }
}
