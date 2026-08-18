use std::collections::BTreeSet;

use regex::Regex;
use serde_json::Value;

use super::example_matches_schema;
use crate::schema::count::{keyword_u64, len_at_least, len_at_most};

pub(super) fn object_matches_schema(
    map: &serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    object_size_matches_schema(map, object)
        && property_names_match_schema(map, object, root, visited_refs)
        && required_properties_match_schema(map, object)
        && fixed_properties_match_schema(map, object, root, visited_refs)
        && dynamic_properties_match_schema(map, object, root, visited_refs)
}

fn object_size_matches_schema(
    map: &serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
) -> bool {
    keyword_u64(object, "minProperties")
        .is_none_or(|min_properties| len_at_least(map.len(), min_properties))
        && keyword_u64(object, "maxProperties")
            .is_none_or(|max_properties| len_at_most(map.len(), max_properties))
}

fn property_names_match_schema(
    map: &serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &BTreeSet<String>,
) -> bool {
    object.get("propertyNames").is_none_or(|property_names| {
        map.keys().all(|key| {
            example_matches_schema(
                &Value::String(key.clone()),
                property_names,
                root,
                &mut visited_refs.clone(),
            )
        })
    })
}

fn required_properties_match_schema(
    map: &serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
) -> bool {
    object
        .get("required")
        .and_then(Value::as_array)
        .is_none_or(|required| {
            required
                .iter()
                .filter_map(Value::as_str)
                .all(|key| map.contains_key(key))
        })
}

fn fixed_properties_match_schema(
    map: &serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        return true;
    };

    properties.iter().all(|(key, child_schema)| {
        map.get(key).is_none_or(|child_value| {
            example_matches_schema(child_value, child_schema, root, visited_refs)
        })
    })
}

fn dynamic_properties_match_schema(
    map: &serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let fixed_properties = object
        .get("properties")
        .and_then(Value::as_object)
        .map_or_else(BTreeSet::new, |properties| {
            properties.keys().cloned().collect::<BTreeSet<_>>()
        });
    let Some(pattern_matched_keys) =
        pattern_properties_match_schema(map, object, root, visited_refs)
    else {
        return false;
    };

    object.get("additionalProperties").is_none_or(|additional| {
        map.iter().all(|(key, child_value)| {
            fixed_properties.contains(key)
                || pattern_matched_keys.contains(key)
                || example_matches_schema(child_value, additional, root, visited_refs)
        })
    })
}

fn pattern_properties_match_schema(
    map: &serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> Option<BTreeSet<String>> {
    let mut matched_keys = BTreeSet::new();
    let Some(pattern_properties) = object.get("patternProperties").and_then(Value::as_object)
    else {
        return Some(matched_keys);
    };

    for (key, child_value) in map {
        for (pattern, child_schema) in pattern_properties {
            if Regex::new(pattern).is_ok_and(|regex| regex.is_match(key)) {
                matched_keys.insert(key.clone());
                if !example_matches_schema(child_value, child_schema, root, visited_refs) {
                    return None;
                }
            }
        }
    }

    Some(matched_keys)
}
