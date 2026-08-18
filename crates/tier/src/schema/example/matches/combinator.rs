use std::collections::BTreeSet;

use serde_json::Value;

use super::example_matches_schema;

pub(super) fn combinators_match_schema(
    value: &Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &BTreeSet<String>,
) -> bool {
    all_of_matches(value, object, root, visited_refs)
        && any_of_matches(value, object, root, visited_refs)
        && one_of_matches(value, object, root, visited_refs)
        && not_matches(value, object, root, visited_refs)
}

fn all_of_matches(
    value: &Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &BTreeSet<String>,
) -> bool {
    object
        .get("allOf")
        .and_then(Value::as_array)
        .is_none_or(|children| {
            children.iter().all(|child| {
                let mut branch_refs = visited_refs.clone();
                example_matches_schema(value, child, root, &mut branch_refs)
            })
        })
}

fn any_of_matches(
    value: &Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &BTreeSet<String>,
) -> bool {
    object
        .get("anyOf")
        .and_then(Value::as_array)
        .is_none_or(|children| {
            children.iter().any(|child| {
                let mut branch_refs = visited_refs.clone();
                example_matches_schema(value, child, root, &mut branch_refs)
            })
        })
}

fn one_of_matches(
    value: &Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &BTreeSet<String>,
) -> bool {
    object
        .get("oneOf")
        .and_then(Value::as_array)
        .is_none_or(|children| {
            children
                .iter()
                .filter(|child| {
                    let mut branch_refs = visited_refs.clone();
                    example_matches_schema(value, child, root, &mut branch_refs)
                })
                .take(2)
                .count()
                == 1
        })
}

fn not_matches(
    value: &Value,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &BTreeSet<String>,
) -> bool {
    object.get("not").is_none_or(|child| {
        let mut branch_refs = visited_refs.clone();
        !example_matches_schema(value, child, root, &mut branch_refs)
    })
}
