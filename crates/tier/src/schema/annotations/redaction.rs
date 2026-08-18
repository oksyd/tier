use serde_json::{Map, Value};

use crate::schema::core::resolve_schema_ref;

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
    match node {
        Value::Object(object) => {
            project_secret_ref_annotations(object, root);
            redact_local_secret_example(object);
            for value in object.values_mut() {
                redact_secret_schema_examples(value, root);
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_secret_schema_examples(value, root);
            }
        }
        _ => {}
    }
}

fn redact_local_secret_example(object: &mut Map<String, Value>) {
    if is_secret_schema_node(object)
        && let Some(example) = object.get_mut("example")
    {
        *example = redact_example_value(example);
    }
}

fn project_secret_ref_annotations(object: &mut Map<String, Value>, root: &Value) {
    let Some(reference) = object.get("$ref").and_then(Value::as_str) else {
        return;
    };
    let Some(target) = resolve_schema_ref(root, reference).and_then(Value::as_object) else {
        return;
    };
    if !is_secret_schema_node(target) {
        return;
    }

    object.insert("writeOnly".to_owned(), Value::Bool(true));
    object.insert("x-tier-secret".to_owned(), Value::Bool(true));
    if !object.contains_key("example")
        && let Some(example) = target.get("example")
    {
        object.insert("example".to_owned(), redact_example_value(example));
    }
}
