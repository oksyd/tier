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

/// Follow recursive schemas alongside the finite input value. Reference cycles
/// at the same value are stopped; descending into a child starts a new chain.
pub(super) fn secret_paths_for_value<T: schemars::JsonSchema>(value: &Value) -> BTreeSet<String> {
    let schema = crate::schema::json_schema_for::<T>();
    let mut paths = BTreeSet::new();
    collect_value_secrets(
        &schema,
        &schema,
        value,
        "",
        &mut paths,
        &mut BTreeSet::new(),
    );
    paths
}

fn collect_value_secrets(
    schema: &Value,
    root: &Value,
    value: &Value,
    path: &str,
    paths: &mut BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) {
    let Some(object) = schema.as_object() else {
        return;
    };
    if ["writeOnly", "x-tier-secret"]
        .iter()
        .any(|key| object.get(*key).and_then(Value::as_bool) == Some(true))
    {
        if !path.is_empty() {
            paths.insert(path.to_owned());
        }
        return;
    }
    if let Some(reference) = object.get("$ref").and_then(Value::as_str)
        && refs.insert(reference.to_owned())
    {
        if let Some(target) = resolve_schema_ref(root, reference) {
            collect_value_secrets(target, root, value, path, paths, refs);
        }
        refs.remove(reference);
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = object.get(key).and_then(Value::as_array) {
            for child in children {
                collect_value_secrets(child, root, value, path, paths, refs);
            }
        }
    }
    match value {
        Value::Object(values) => {
            for (key, value) in values {
                let next = join_path(path, key);
                let mut matched = false;
                if let Some(child) = object.get("properties").and_then(|props| props.get(key)) {
                    matched = true;
                    collect_value_secrets(child, root, value, &next, paths, &mut BTreeSet::new());
                }
                if let Some(patterns) = object.get("patternProperties").and_then(Value::as_object) {
                    for (pattern, child) in patterns {
                        if regex::Regex::new(pattern).is_ok_and(|re| re.is_match(key)) {
                            matched = true;
                            collect_value_secrets(
                                child,
                                root,
                                value,
                                &next,
                                paths,
                                &mut BTreeSet::new(),
                            );
                        }
                    }
                }
                if !matched && let Some(child) = object.get("additionalProperties") {
                    collect_value_secrets(child, root, value, &next, paths, &mut BTreeSet::new());
                }
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                let next = join_path(path, &index.to_string());
                let tuple = object
                    .get("prefixItems")
                    .or_else(|| object.get("items").filter(|v| v.is_array()));
                let child = tuple
                    .and_then(Value::as_array)
                    .and_then(|items| items.get(index))
                    .or_else(|| object.get("items").filter(|v| !v.is_array()))
                    .or_else(|| legacy_additional_items_for_schema(object));
                if let Some(child) = child {
                    collect_value_secrets(child, root, value, &next, paths, &mut BTreeSet::new());
                }
                if let Some(child) = object.get("contains") {
                    collect_value_secrets(child, root, value, &next, paths, &mut BTreeSet::new());
                }
            }
        }
        _ => {}
    }
}
