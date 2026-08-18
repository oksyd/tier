use std::collections::BTreeSet;

use serde_json::Value;

use super::annotations::{is_secret_schema_node, redact_example_value};
use super::core::{inlined_schema_ref, merged_object_level_property_names, schema_preferred_type};

mod array;
mod build_array;
mod build_combinator;
mod build_object;
mod fallback;
mod matches;
mod merge;
mod numeric;
mod object;

pub(crate) use self::array::{
    allows_additional_array_items_for_schema, legacy_additional_items_for_schema,
    required_contains_additional_items_for_docs,
};
use self::array::{uniquify_example_value_in_place, uniquify_merged_array_example};
use self::build_array::{merge_array_examples, merge_contains_array_examples};
use self::build_combinator::{merge_all_of_examples, merge_first_non_null_branch_example};
use self::build_object::{merge_additional_property_examples, merge_object_examples};
use self::fallback::fallback_string_example;
use self::matches::example_matches_schema;
use self::merge::merge_example_value;
use self::numeric::{fallback_integer_example, fallback_number_example};
pub(crate) use self::object::dynamic_object_placeholder_for_schema;

pub(super) fn build_example_value(
    schema: &Value,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
    scope_reserved_keys: Option<&BTreeSet<String>>,
) -> Option<Value> {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if visited_refs.insert(reference.to_owned()) {
            let inlined = inlined_schema_ref(schema, root)?;
            let example = build_example_value(&inlined, root, visited_refs, scope_reserved_keys);
            visited_refs.remove(reference);
            return example;
        }
        return None;
    }

    match schema {
        Value::Bool(true) => return Some(Value::Null),
        Value::Bool(false) => return None,
        _ => {}
    }

    let object = schema.as_object()?;
    let is_secret = is_secret_schema_node(object);
    let reserved_keys = merged_object_level_property_names(schema, root, scope_reserved_keys);

    if let Some(constant) = object.get("const") {
        return value_satisfies_schema(constant, schema, root)
            .then(|| redact_if_secret(constant, is_secret));
    }

    if let Some(example) = valid_schema_annotation("example", schema, object, root) {
        return Some(redact_if_secret(example, is_secret));
    }

    if let Some(default) = valid_schema_annotation("default", schema, object, root) {
        return Some(redact_if_secret(default, is_secret));
    }

    let mut merged = None;
    merge_all_of_examples(
        schema,
        object,
        root,
        visited_refs,
        &reserved_keys,
        &mut merged,
    );

    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        return values
            .iter()
            .find(|value| value_satisfies_schema(value, schema, root))
            .map(|value| redact_if_secret(value, is_secret));
    }

    merge_first_non_null_branch_example(
        "oneOf",
        schema,
        object,
        root,
        visited_refs,
        &reserved_keys,
        &mut merged,
    );
    merge_first_non_null_branch_example(
        "anyOf",
        schema,
        object,
        root,
        visited_refs,
        &reserved_keys,
        &mut merged,
    );
    merge_object_examples(object, root, visited_refs, &reserved_keys, &mut merged);
    merge_array_examples(object, root, visited_refs, &mut merged);

    uniquify_merged_array_example(&mut merged, object, root);
    merge_contains_array_examples(object, root, visited_refs, &mut merged);
    merge_additional_property_examples(object, root, visited_refs, &reserved_keys, &mut merged);

    if let Some(mut merged) = merged {
        uniquify_example_value_in_place(&mut merged, object, root);
        return Some(redact_if_secret(&merged, is_secret));
    }

    let fallback = match schema_preferred_type(object).as_str() {
        "string" => fallback_string_example(object, |candidate| {
            value_satisfies_schema(&Value::String(candidate.to_owned()), schema, root)
        })
        .map(Value::String),
        "integer" => fallback_integer_example(object, |candidate| {
            value_satisfies_schema(&Value::Number(candidate.clone()), schema, root)
        })
        .map(Value::Number),
        "number" => fallback_number_example(object, |candidate| {
            value_satisfies_schema(&Value::Number(candidate.clone()), schema, root)
        })
        .map(Value::Number),
        "boolean" => fallback_boolean_example(schema, root),
        "array" => valid_fallback(Value::Array(Vec::new()), schema, root),
        "object" => valid_fallback(Value::Object(Default::default()), schema, root),
        _ => Some(Value::Null),
    }?;

    Some(if is_secret {
        redact_example_value(&fallback)
    } else {
        fallback
    })
}

fn fallback_boolean_example(schema: &Value, root: &Value) -> Option<Value> {
    [false, true]
        .into_iter()
        .find_map(|candidate| valid_fallback(Value::Bool(candidate), schema, root))
}

fn valid_fallback(value: Value, schema: &Value, root: &Value) -> Option<Value> {
    value_satisfies_schema(&value, schema, root).then_some(value)
}

fn valid_schema_annotation<'a>(
    key: &str,
    schema: &Value,
    object: &'a serde_json::Map<String, Value>,
    root: &Value,
) -> Option<&'a Value> {
    object
        .get(key)
        .filter(|value| value_satisfies_schema(value, schema, root))
}

fn value_satisfies_schema(value: &Value, schema: &Value, root: &Value) -> bool {
    let mut visited_refs = BTreeSet::new();
    example_matches_schema(value, schema, root, &mut visited_refs)
}

fn redact_if_secret(value: &Value, is_secret: bool) -> Value {
    if is_secret {
        redact_example_value(value)
    } else {
        value.clone()
    }
}
