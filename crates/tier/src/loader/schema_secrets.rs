use std::collections::BTreeSet;

use serde_json::Value;

use crate::path::join_path;
use crate::schema::{legacy_additional_items_for_schema, resolve_schema_ref};

pub(super) fn schema_secret_paths<T>() -> BTreeSet<String>
where
    T: schemars::JsonSchema,
{
    let schema = crate::schema::json_schema_for::<T>();
    let mut paths = BTreeSet::new();
    collect_secret_paths_from_schema(&schema, &schema, "", &mut paths, &mut BTreeSet::new());
    paths
}

fn collect_secret_paths_from_schema(
    schema: &Value,
    root: &Value,
    current: &str,
    paths: &mut BTreeSet<String>,
    visited_refs: &mut BTreeSet<String>,
) {
    let Some(object) = schema.as_object() else {
        return;
    };

    let is_secret = object
        .get("x-tier-secret")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || object
            .get("writeOnly")
            .and_then(Value::as_bool)
            .unwrap_or(false);

    if is_secret && !current.is_empty() {
        paths.insert(current.to_owned());
    }

    if let Some(reference) = object.get("$ref").and_then(Value::as_str)
        && visited_refs.insert(reference.to_owned())
        && let Some(target) = resolve_schema_ref(root, reference)
    {
        collect_secret_paths_from_schema(target, root, current, paths, visited_refs);
        visited_refs.remove(reference);
    }

    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (key, child) in properties {
            let next = join_path(current, key);
            collect_secret_paths_from_schema(child, root, &next, paths, visited_refs);
        }
    }

    if let Some(pattern_properties) = object.get("patternProperties").and_then(Value::as_object) {
        let next = join_path(current, "*");
        for child in pattern_properties.values() {
            collect_secret_paths_from_schema(child, root, &next, paths, visited_refs);
        }
    }

    if let Some(items) = object.get("prefixItems").and_then(Value::as_array) {
        for (index, child) in items.iter().enumerate() {
            let next = join_path(current, &index.to_string());
            collect_secret_paths_from_schema(child, root, &next, paths, visited_refs);
        }
    }

    if let Some(items) = object.get("items").and_then(Value::as_array) {
        for (index, child) in items.iter().enumerate() {
            let next = join_path(current, &index.to_string());
            collect_secret_paths_from_schema(child, root, &next, paths, visited_refs);
        }
    }

    if let Some(items) = object.get("items").filter(|value| !value.is_array()) {
        let next = join_path(current, "*");
        collect_secret_paths_from_schema(items, root, &next, paths, visited_refs);
    }

    if let Some(additional_items) = legacy_additional_items_for_schema(object) {
        let next = join_path(current, "*");
        collect_secret_paths_from_schema(additional_items, root, &next, paths, visited_refs);
    }

    if let Some(additional) = object
        .get("additionalProperties")
        .filter(|value| value.is_object())
    {
        let next = join_path(current, "*");
        collect_secret_paths_from_schema(additional, root, &next, paths, visited_refs);
    }

    if let Some(contains) = object.get("contains") {
        let next = join_path(current, "*");
        collect_secret_paths_from_schema(contains, root, &next, paths, visited_refs);
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(array) = object.get(keyword).and_then(Value::as_array) {
            for child in array {
                collect_secret_paths_from_schema(child, root, current, paths, visited_refs);
            }
        }
    }
}
