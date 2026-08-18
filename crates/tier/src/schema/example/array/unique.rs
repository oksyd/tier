use std::collections::BTreeSet;

use serde_json::Value;

mod generic;
mod number;
mod string;

use self::generic::uniquify_generic_example_value;
use self::number::uniquify_number_example;
use self::string::uniquify_string_example;
use super::super::matches::example_matches_schema;
use super::constraints::{array_requires_unique_items, legacy_additional_items_for_schema};
use crate::value::values_contain;

pub(in crate::schema::example) fn uniquify_merged_array_example(
    merged: &mut Option<Value>,
    object: &serde_json::Map<String, Value>,
    root: &Value,
) {
    if let Some(value) = merged {
        uniquify_example_value_in_place(value, object, root);
    }
}

pub(in crate::schema::example) fn uniquify_example_value_in_place(
    value: &mut Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
) {
    if let Value::Array(items) = value
        && array_requires_unique_items(object)
    {
        uniquify_array_example_items(items, object, root);
    }
}

fn array_item_schema(object: &serde_json::Map<String, Value>, index: usize) -> Option<&Value> {
    if let Some(prefix_items) = object.get("prefixItems").and_then(Value::as_array)
        && let Some(schema) = prefix_items.get(index)
    {
        return Some(schema);
    }

    if let Some(item_schemas) = object.get("items").and_then(Value::as_array)
        && let Some(schema) = item_schemas.get(index)
    {
        return Some(schema);
    }

    object
        .get("items")
        .filter(|value| !value.is_array() && !matches!(value, Value::Bool(false)))
        .or_else(|| {
            legacy_additional_items_for_schema(object)
                .filter(|value| !value.is_array() && !matches!(value, Value::Bool(false)))
        })
}

pub(super) fn uniquify_array_example_items(
    items: &mut [Value],
    object: &serde_json::Map<String, Value>,
    root: &Value,
) {
    let mut seen = Vec::<Value>::new();
    for index in 0..items.len() {
        let Some((item, future)) = items[index..].split_first_mut() else {
            break;
        };
        if values_contain(&seen, item)
            && let Some(schema) = array_item_schema(object, index)
        {
            let mut reserved = seen.clone();
            reserved.extend(future.iter().cloned());
            let unique = uniquify_example_value(item.clone(), schema, root, &reserved)
                .or_else(|| uniquify_example_value(item.clone(), schema, root, &seen));
            if let Some(unique) = unique {
                *item = unique;
            }
        }
        seen.push(item.clone());
    }
}

pub(in crate::schema::example) fn build_repeated_example_values(
    example: Value,
    schema: &Value,
    root: &Value,
    count: usize,
    unique_items: bool,
    existing: &[Value],
) -> Vec<Value> {
    let mut taken = if unique_items {
        existing.to_vec()
    } else {
        Vec::new()
    };
    let mut rendered = Vec::with_capacity(count);
    for _ in 0..count {
        let value = if unique_items {
            uniquify_example_value(example.clone(), schema, root, &taken)
                .unwrap_or_else(|| example.clone())
        } else {
            example.clone()
        };
        if unique_items {
            taken.push(value.clone());
        }
        rendered.push(value);
    }
    rendered
}

fn uniquify_example_value(
    value: Value,
    schema: &Value,
    root: &Value,
    existing: &[Value],
) -> Option<Value> {
    if !values_contain(existing, &value) {
        return Some(value);
    }

    let specialized = match value {
        Value::String(text) => uniquify_string_example(text, schema, root, existing),
        Value::Number(number) => uniquify_number_example(number, schema, root, existing),
        Value::Bool(flag) => {
            let candidate = Value::Bool(!flag);
            (!values_contain(existing, &candidate)
                && example_matches_schema(&candidate, schema, root, &mut BTreeSet::new()))
            .then_some(candidate)
        }
        _ => None,
    };

    specialized.or_else(|| uniquify_generic_example_value(schema, root, existing))
}
