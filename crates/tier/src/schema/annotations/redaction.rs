use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::schema::core::resolve_schema_ref;

const SCHEMA_MAP_KEYWORDS: [&str; 6] = [
    "properties",
    "patternProperties",
    "$defs",
    "definitions",
    "dependentSchemas",
    "dependencies",
];
const SCHEMA_KEYWORDS: [&str; 16] = [
    "allOf",
    "anyOf",
    "oneOf",
    "prefixItems",
    "items",
    "additionalItems",
    "additionalProperties",
    "contains",
    "propertyNames",
    "not",
    "if",
    "then",
    "else",
    "unevaluatedItems",
    "unevaluatedProperties",
    "contentSchema",
];

pub(in crate::schema) fn is_secret_schema_node(object: &Map<String, Value>) -> bool {
    object
        .get("writeOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || object
            .get("x-tier-secret")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

pub(in crate::schema) fn redact_example_value(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(redact_example_value).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), redact_example_value(value)))
                .collect(),
        ),
        _ => Value::String("<secret>".to_owned()),
    }
}

pub(in crate::schema) fn redact_secret_schema_examples(node: &mut Value, root: &Value) {
    let mut secret_refs = BTreeSet::new();
    redact_schema_node(node, root, false, &mut secret_refs);
    let mut processed_refs = BTreeSet::new();
    while let Some(reference) = secret_refs.pop_first() {
        if !processed_refs.insert(reference.clone()) {
            continue;
        }
        if let Some(target) = reference
            .strip_prefix('#')
            .and_then(|pointer| node.pointer_mut(pointer))
        {
            // A reference retains its definition in the exported document. Redact
            // its annotations too, including definitions reached recursively.
            redact_schema_node(target, root, true, &mut secret_refs);
        }
    }
    remove_unused_definitions(node);
}

fn remove_unused_definitions(root: &mut Value) {
    // Metadata traversal inlines path-specific references. Their unused original
    // definitions must not retain unredacted copies of defaults or examples.
    let mut pending = BTreeSet::new();
    collect_schema_refs(root, &mut pending);
    let mut referenced = BTreeSet::new();
    while let Some(reference) = pending.pop_first() {
        if referenced.insert(reference.clone())
            && let Some(target) = resolve_schema_ref(root, &reference)
        {
            collect_schema_refs(target, &mut pending);
        }
    }
    for keyword in ["$defs", "definitions"] {
        if let Some(definitions) = root.get_mut(keyword).and_then(Value::as_object_mut) {
            definitions.retain(|name, _| {
                let pointer = format!("#/{keyword}/{}", name.replace('~', "~0").replace('/', "~1"));
                referenced.iter().any(|reference| {
                    reference == &pointer
                        || reference
                            .strip_prefix(&pointer)
                            .is_some_and(|suffix| suffix.starts_with('/'))
                })
            });
        }
    }
}

fn collect_schema_refs(node: &Value, references: &mut BTreeSet<String>) {
    let Some(object) = node.as_object() else {
        return;
    };
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        references.insert(reference.to_owned());
    }
    for keyword in SCHEMA_MAP_KEYWORDS {
        if !matches!(keyword, "$defs" | "definitions")
            && let Some(children) = object.get(keyword).and_then(Value::as_object)
        {
            for child in children.values() {
                collect_schema_refs(child, references);
            }
        }
    }
    for keyword in SCHEMA_KEYWORDS {
        if let Some(child) = object.get(keyword) {
            if let Some(children) = child.as_array() {
                for child in children {
                    collect_schema_refs(child, references);
                }
            } else {
                collect_schema_refs(child, references);
            }
        }
    }
}

fn redact_schema_node(
    node: &mut Value,
    root: &Value,
    inherited_secret: bool,
    secret_refs: &mut BTreeSet<String>,
) {
    let Some(object) = node.as_object_mut() else {
        return;
    };
    project_secret_ref_annotations(object, root);
    let secret = inherited_secret || is_secret_schema_node(object);
    if secret && let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        secret_refs.insert(reference.to_owned());
    }
    let schema = Value::Object(object.clone());
    for keyword in ["default", "example", "examples"] {
        let Some(annotation) = object.get_mut(keyword) else {
            continue;
        };
        if secret {
            *annotation = redact_example_value(annotation);
        } else if keyword == "examples" {
            if let Some(examples) = annotation.as_array_mut() {
                for example in examples {
                    redact_value_for_schema(example, &schema, root, &mut BTreeSet::new());
                }
            }
        } else {
            redact_value_for_schema(annotation, &schema, root, &mut BTreeSet::new());
        }
    }

    // Visit schema positions only: objects within example/default data are not schemas.
    for keyword in SCHEMA_MAP_KEYWORDS {
        if let Some(children) = object.get_mut(keyword).and_then(Value::as_object_mut) {
            for child in children.values_mut() {
                redact_schema_node(child, root, secret, secret_refs);
            }
        }
    }
    for keyword in SCHEMA_KEYWORDS {
        if let Some(child) = object.get_mut(keyword) {
            if let Some(children) = child.as_array_mut() {
                for child in children {
                    redact_schema_node(child, root, secret, secret_refs);
                }
            } else {
                redact_schema_node(child, root, secret, secret_refs);
            }
        }
    }
}

fn redact_value_for_schema(
    value: &mut Value,
    schema: &Value,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) {
    let Some(object) = schema.as_object() else {
        return;
    };
    if is_secret_schema_node(object) {
        *value = redact_example_value(value);
        return;
    }
    if let Some(reference) = object.get("$ref").and_then(Value::as_str)
        && visited_refs.insert(reference.to_owned())
    {
        if let Some(target) = resolve_schema_ref(root, reference) {
            redact_value_for_schema(value, target, root, visited_refs);
        }
        visited_refs.remove(reference);
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            for child in children {
                redact_value_for_schema(value, child, root, visited_refs);
            }
        }
    }
    for keyword in ["if", "then", "else"] {
        if let Some(child) = object.get(keyword) {
            redact_value_for_schema(value, child, root, visited_refs);
        }
    }
    match value {
        Value::Object(values) => {
            let properties = object.get("properties").and_then(Value::as_object);
            let patterns = object.get("patternProperties").and_then(Value::as_object);
            for (name, value) in values {
                let mut matched = false;
                if let Some(child) = properties.and_then(|properties| properties.get(name)) {
                    matched = true;
                    redact_value_for_schema(value, child, root, &mut BTreeSet::new());
                }
                if let Some(patterns) = patterns {
                    for (pattern, child) in patterns {
                        if regex::Regex::new(pattern).is_ok_and(|regex| regex.is_match(name)) {
                            matched = true;
                            redact_value_for_schema(value, child, root, &mut BTreeSet::new());
                        }
                    }
                }
                if !matched && let Some(child) = object.get("additionalProperties") {
                    redact_value_for_schema(value, child, root, &mut BTreeSet::new());
                }
            }
        }
        Value::Array(values) => {
            let tuple_items = object
                .get("prefixItems")
                .or_else(|| object.get("items").filter(|items| items.is_array()))
                .and_then(Value::as_array);
            let remaining = if object.get("items").is_some_and(Value::is_array) {
                object.get("additionalItems")
            } else {
                object.get("items")
            };
            for (index, value) in values.iter_mut().enumerate() {
                if let Some(child) = tuple_items.and_then(|items| items.get(index)).or(remaining) {
                    redact_value_for_schema(value, child, root, &mut BTreeSet::new());
                }
                if let Some(child) = object.get("contains") {
                    redact_value_for_schema(value, child, root, &mut BTreeSet::new());
                }
            }
        }
        _ => {}
    }
}

fn project_secret_ref_annotations(object: &mut Map<String, Value>, root: &Value) {
    let Some(reference) = object.get("$ref").and_then(Value::as_str) else {
        return;
    };
    let Some(target) = resolve_schema_ref(root, reference).and_then(Value::as_object) else {
        return;
    };
    if !is_secret_schema_node(target) && !is_secret_schema_node(object) {
        return;
    }

    object.insert("writeOnly".to_owned(), Value::Bool(true));
    object.insert("x-tier-secret".to_owned(), Value::Bool(true));
    for keyword in ["default", "example", "examples"] {
        if !object.contains_key(keyword)
            && let Some(annotation) = target.get(keyword)
        {
            object.insert(keyword.to_owned(), redact_example_value(annotation));
        }
    }
}
