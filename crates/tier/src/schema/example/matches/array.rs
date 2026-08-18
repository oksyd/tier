use std::collections::BTreeSet;

use serde_json::Value;

use super::example_matches_schema;
use crate::schema::count::{keyword_u64, len_at_least, len_at_most};
use crate::schema::example::array::{
    array_requires_unique_items, legacy_additional_items_for_schema,
};
use crate::value::values_contain;

pub(super) fn array_matches_schema(
    items: &[Value],
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    array_size_matches_schema(items, object)
        && array_unique_items_match_schema(items, object)
        && fixed_items_match_schema(items, object, root, visited_refs)
        && additional_items_match_schema(items, object, root, visited_refs)
        && contains_matches_schema(items, object, root, visited_refs)
}

fn array_size_matches_schema(items: &[Value], object: &serde_json::Map<String, Value>) -> bool {
    keyword_u64(object, "minItems").is_none_or(|min_items| len_at_least(items.len(), min_items))
        && keyword_u64(object, "maxItems")
            .is_none_or(|max_items| len_at_most(items.len(), max_items))
}

fn array_unique_items_match_schema(
    items: &[Value],
    object: &serde_json::Map<String, Value>,
) -> bool {
    if !array_requires_unique_items(object) {
        return true;
    }

    let mut seen = Vec::<Value>::new();
    for item in items {
        if values_contain(&seen, item) {
            return false;
        }
        seen.push(item.clone());
    }
    true
}

fn fixed_items_match_schema(
    items: &[Value],
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    fixed_items_for_schema(object).all(|(index, child_schema)| {
        items.get(index).is_none_or(|child_value| {
            example_matches_schema(child_value, child_schema, root, visited_refs)
        })
    })
}

fn additional_items_match_schema(
    items: &[Value],
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let fixed_item_count = fixed_item_count(object);

    if let Some(items_schema) = object.get("items").filter(|value| !value.is_array()) {
        for child_value in items.iter().skip(fixed_item_count) {
            if !example_matches_schema(child_value, items_schema, root, visited_refs) {
                return false;
            }
        }
    }

    if let Some(additional_schema) =
        legacy_additional_items_for_schema(object).filter(|value| !value.is_array())
    {
        for child_value in items.iter().skip(fixed_item_count) {
            if !example_matches_schema(child_value, additional_schema, root, visited_refs) {
                return false;
            }
        }
    }

    true
}

fn contains_matches_schema(
    items: &[Value],
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let Some(contains_schema) = object.get("contains") else {
        return true;
    };

    let matching_items = items
        .iter()
        .filter(|child_value| {
            example_matches_schema(child_value, contains_schema, root, visited_refs)
        })
        .count();
    let min_contains = keyword_u64(object, "minContains").unwrap_or(1);
    let max_contains = keyword_u64(object, "maxContains");
    len_at_least(matching_items, min_contains)
        && max_contains.is_none_or(|max_contains| len_at_most(matching_items, max_contains))
}

fn fixed_items_for_schema(
    object: &serde_json::Map<String, Value>,
) -> impl Iterator<Item = (usize, &Value)> {
    let prefix_items = object
        .get("prefixItems")
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter().enumerate());
    let tuple_items = object
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter().enumerate());

    prefix_items.chain(tuple_items)
}

fn fixed_item_count(object: &serde_json::Map<String, Value>) -> usize {
    object
        .get("prefixItems")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
        .max(
            object
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
        )
}
