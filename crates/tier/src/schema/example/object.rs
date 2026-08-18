use std::collections::BTreeSet;

use regex::Regex;
use serde_json::Value;

use super::super::core::dynamic_object_placeholder;
use super::matches::example_matches_schema;
use crate::schema::count::{keyword_u64, usize_saturating};

const COMMON_PROPERTY_NAME_CANDIDATES: &[&str] = &[
    "0",
    "1",
    "42",
    "000",
    "001",
    "123",
    "999",
    "item",
    "key",
    "entry",
    "value",
    "name",
    "field",
    "property",
    "primary",
    "secondary",
    "example",
    "default",
    "token",
    "service",
    "svc-0",
    "svc-1",
    "svc-42",
    "A",
    "B",
    "X",
    "ID",
    "KEY",
    "ITEM",
    "TOKEN",
    "SERVICE",
    "PRIMARY",
    "SECONDARY",
    "EXAMPLE",
    "DEFAULT",
];

const PATTERN_SUFFIX_CANDIDATES: &[&str] = &[
    "", "0", "1", "42", "000", "001", "123", "999", "item", "key", "entry", "value", "example",
    "name", "field", "token", "service", "A", "B", "X", "ID", "KEY", "ITEM", "TOKEN", "SERVICE",
];

pub(crate) fn dynamic_object_placeholder_for_schema(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    reserved: &BTreeSet<String>,
) -> Option<String> {
    let property_names = object.get("propertyNames")?;
    let mut candidates = property_name_candidates(property_names);
    if candidates.is_empty() {
        extend_common_property_name_candidates(&mut candidates);
    }

    let mut taken = reserved.clone();
    for candidate in candidates {
        if taken.contains(&candidate) {
            continue;
        }
        if property_name_matches_schema(&candidate, property_names, root) {
            return Some(candidate);
        }
        taken.insert(candidate);
    }

    let mut index = 0usize;
    loop {
        for stem in ["item", "key", "entry", "value", "field", "name"] {
            let candidate = format!("{stem}_{index}");
            if reserved.contains(&candidate) {
                continue;
            }
            if property_name_matches_schema(&candidate, property_names, root) {
                return Some(candidate);
            }
        }
        if index > 1024 {
            break;
        }
        index += 1;
    }

    None
}

pub(super) fn dynamic_object_placeholders_for_schema(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    reserved: &BTreeSet<String>,
    count: usize,
) -> Vec<String> {
    let mut taken = reserved.clone();
    let mut placeholders = Vec::with_capacity(count);
    let constrained_by_property_names = object.contains_key("propertyNames");
    for _ in 0..count {
        let placeholder = if constrained_by_property_names {
            match dynamic_object_placeholder_for_schema(object, root, &taken) {
                Some(placeholder) => placeholder,
                None => break,
            }
        } else {
            dynamic_object_placeholder(&taken)
        };
        taken.insert(placeholder.clone());
        placeholders.push(placeholder);
    }
    placeholders
}

pub(super) fn pattern_property_placeholder_for_schema(
    pattern: &str,
    object: &serde_json::Map<String, Value>,
    root: &Value,
    reserved: &BTreeSet<String>,
) -> Option<String> {
    let regex = Regex::new(pattern).ok()?;
    let prefix = literal_regex_prefix(pattern);
    let property_names = object.get("propertyNames");

    let mut candidates = Vec::new();
    if let Some(property_names) = property_names {
        candidates.extend(property_name_candidates(property_names));
    }
    extend_pattern_property_candidates(&mut candidates, prefix.as_deref());

    for candidate in candidates {
        if !reserved.contains(&candidate)
            && regex.is_match(&candidate)
            && property_names.is_none_or(|property_names| {
                property_name_matches_schema(&candidate, property_names, root)
            })
        {
            return Some(candidate);
        }
    }

    for index in 0..1024 {
        for stem in ["item", "key", "entry", "value", "example", "name", "field"] {
            let candidate = match prefix.as_deref() {
                Some(prefix) if !prefix.is_empty() => format!("{prefix}{stem}{index}"),
                _ => format!("{stem}_{index}"),
            };
            if !reserved.contains(&candidate)
                && regex.is_match(&candidate)
                && property_names.is_none_or(|property_names| {
                    property_name_matches_schema(&candidate, property_names, root)
                })
            {
                return Some(candidate);
            }
        }
    }

    None
}

pub(super) fn trim_object_example_properties(
    rendered: &mut serde_json::Map<String, Value>,
    object: &serde_json::Map<String, Value>,
) {
    let Some(max_properties) = keyword_u64(object, "maxProperties").map(usize_saturating) else {
        return;
    };

    if rendered.len() <= max_properties {
        return;
    }

    let required = object
        .get("required")
        .and_then(Value::as_array)
        .map(|required| {
            required
                .iter()
                .filter_map(Value::as_str)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    let required_present = rendered
        .keys()
        .filter(|key| required.contains(key.as_str()))
        .count();
    if required_present >= max_properties {
        rendered.retain(|key, _| required.contains(key.as_str()));
        return;
    }

    let mut optional_slots = max_properties - required_present;
    rendered.retain(|key, _| {
        if required.contains(key.as_str()) {
            true
        } else if optional_slots > 0 {
            optional_slots -= 1;
            true
        } else {
            false
        }
    });
}

fn literal_regex_prefix(pattern: &str) -> Option<String> {
    let pattern = pattern.strip_prefix('^').unwrap_or(pattern);
    let mut prefix = String::new();
    let mut escaped = false;

    for ch in pattern.chars() {
        if escaped {
            if ch.is_ascii_alphanumeric() {
                break;
            }
            prefix.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '$' | '.' | '*' | '+' | '?' | '|' | '(' | '[' | '{' => break,
            _ => prefix.push(ch),
        }
    }

    Some(prefix)
}

fn property_name_candidates(schema: &Value) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(constant) = schema.get("const").and_then(Value::as_str) {
        candidates.push(constant.to_owned());
    }

    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        candidates.extend(
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned),
        );
    }

    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
        extend_pattern_property_candidates(
            &mut candidates,
            literal_regex_prefix(pattern).as_deref(),
        );
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn extend_pattern_property_candidates(candidates: &mut Vec<String>, prefix: Option<&str>) {
    if let Some(prefix) = prefix.filter(|prefix| !prefix.is_empty()) {
        candidates.extend(
            PATTERN_SUFFIX_CANDIDATES
                .iter()
                .map(|suffix| format!("{prefix}{suffix}")),
        );
    }
    extend_common_property_name_candidates(candidates);
}

fn extend_common_property_name_candidates(candidates: &mut Vec<String>) {
    candidates.extend(
        COMMON_PROPERTY_NAME_CANDIDATES
            .iter()
            .map(|candidate| (*candidate).to_owned()),
    );
}

fn property_name_matches_schema(candidate: &str, schema: &Value, root: &Value) -> bool {
    example_matches_schema(
        &Value::String(candidate.to_owned()),
        schema,
        root,
        &mut BTreeSet::new(),
    )
}
