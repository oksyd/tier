use std::collections::BTreeSet;

use serde_json::Value;

use super::super::super::matches::example_matches_schema;
use crate::value::values_contain;

pub(super) fn uniquify_string_example(
    text: String,
    schema: &Value,
    root: &Value,
    existing: &[Value],
) -> Option<Value> {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        for value in values {
            let candidate = value.clone();
            if !values_contain(existing, &candidate)
                && example_matches_schema(&candidate, schema, root, &mut BTreeSet::new())
            {
                return Some(candidate);
            }
        }
    }

    for candidate in constrained_string_unique_candidates(&text, schema) {
        if !values_contain(existing, &candidate)
            && example_matches_schema(&candidate, schema, root, &mut BTreeSet::new())
        {
            return Some(candidate);
        }
    }

    for attempt in 1..=1024 {
        for candidate in [
            Value::String(format!("{text}-{attempt}")),
            Value::String(format!("{text}_{attempt}")),
            Value::String(format!("item{attempt}")),
            Value::String(format!("value{attempt}")),
        ] {
            if !values_contain(existing, &candidate)
                && example_matches_schema(&candidate, schema, root, &mut BTreeSet::new())
            {
                return Some(candidate);
            }
        }
    }

    None
}

fn constrained_string_unique_candidates(text: &str, schema: &Value) -> Vec<Value> {
    let min_length = schema
        .get("minLength")
        .and_then(Value::as_u64)
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0);
    let max_length = schema
        .get("maxLength")
        .and_then(Value::as_u64)
        .and_then(|length| usize::try_from(length).ok());

    let mut lengths = BTreeSet::new();
    let text_length = text.chars().count();
    if text_length > 0 {
        lengths.insert(text_length);
    }
    if min_length > 0 {
        lengths.insert(min_length);
    }
    if let Some(max_length) = max_length
        && max_length > 0
    {
        lengths.insert(max_length);
    }

    let mut candidates = Vec::new();
    for length in lengths.into_iter().take(8) {
        if max_length.is_some_and(|max_length| length > max_length) || length < min_length {
            continue;
        }
        for seed in ['a', 'b', 'c', 'x', 'y', 'z', '0', '1', '2', 'A', 'B', 'C'] {
            let candidate = std::iter::repeat_n(seed, length).collect::<String>();
            if candidate != text {
                candidates.push(Value::String(candidate));
            }
        }
    }

    candidates
}
