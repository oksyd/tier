use std::collections::BTreeSet;

use serde_json::Value;

use crate::number::json_number_from_i128;
use crate::value::values_contain;

use super::super::super::matches::example_matches_schema;
use super::super::super::numeric::{
    first_integer_multiple_at_or_above, integer_multiple_step_constraint,
    last_integer_multiple_at_or_below,
};

pub(super) fn uniquify_number_example(
    number: serde_json::Number,
    schema: &Value,
    root: &Value,
    existing: &[Value],
) -> Option<Value> {
    if let Some(integer) = number.as_i64() {
        return uniquify_integer_number_example(i128::from(integer), schema, root, existing);
    }

    if let Some(integer) = number.as_u64() {
        return uniquify_integer_number_example(i128::from(integer), schema, root, existing);
    }

    let base = number.as_f64()?;
    let step = schema
        .get("multipleOf")
        .and_then(Value::as_f64)
        .filter(|step| step.is_normal() && *step > 0.0)
        .unwrap_or(1.0);
    for attempt in 1..=1024u32 {
        for direction in [1.0, -1.0] {
            let Some(candidate) =
                unique_real_number_candidate(base, step, attempt, direction).map(Value::Number)
            else {
                continue;
            };
            if !values_contain(existing, &candidate)
                && example_matches_schema(&candidate, schema, root, &mut BTreeSet::new())
            {
                return Some(candidate);
            }
        }
    }

    None
}

fn unique_real_number_candidate(
    base: f64,
    step: f64,
    attempt: u32,
    direction: f64,
) -> Option<serde_json::Number> {
    let candidate = base + step * f64::from(attempt) * direction;
    serde_json::Number::from_f64(candidate)
}

fn uniquify_integer_number_example(
    integer: i128,
    schema: &Value,
    root: &Value,
    existing: &[Value],
) -> Option<Value> {
    if let Some(object) = schema.as_object()
        && let Some(step) = integer_multiple_step_constraint(object)
    {
        return uniquify_stepped_integer_number_example(integer, step, schema, root, existing);
    }

    uniquify_adjacent_integer_number_example(integer, schema, root, existing)
}

fn uniquify_stepped_integer_number_example(
    integer: i128,
    step: i128,
    schema: &Value,
    root: &Value,
    existing: &[Value],
) -> Option<Value> {
    let mut next = integer
        .checked_add(1)
        .and_then(|lower| first_integer_multiple_at_or_above(lower, step));
    let mut previous = integer
        .checked_sub(1)
        .and_then(|upper| last_integer_multiple_at_or_below(upper, step));

    for _ in 0..1024 {
        if let Some(value) = next
            .and_then(json_number_from_i128)
            .map(Value::Number)
            .filter(|value| {
                !values_contain(existing, value)
                    && example_matches_schema(value, schema, root, &mut BTreeSet::new())
            })
        {
            return Some(value);
        }
        if let Some(value) = previous
            .and_then(json_number_from_i128)
            .map(Value::Number)
            .filter(|value| {
                !values_contain(existing, value)
                    && example_matches_schema(value, schema, root, &mut BTreeSet::new())
            })
        {
            return Some(value);
        }

        next = next.and_then(|candidate| candidate.checked_add(step));
        previous = previous.and_then(|candidate| candidate.checked_sub(step));
    }
    None
}

fn uniquify_adjacent_integer_number_example(
    integer: i128,
    schema: &Value,
    root: &Value,
    existing: &[Value],
) -> Option<Value> {
    let mut candidate = integer;
    for _ in 0..1024 {
        let Some(next) = candidate.checked_add(1) else {
            break;
        };
        candidate = next;
        if let Some(value) = json_number_from_i128(candidate).map(Value::Number)
            && !values_contain(existing, &value)
            && example_matches_schema(&value, schema, root, &mut BTreeSet::new())
        {
            return Some(value);
        }
    }

    let mut candidate = integer;
    for _ in 0..1024 {
        let Some(previous) = candidate.checked_sub(1) else {
            break;
        };
        candidate = previous;
        if let Some(value) = json_number_from_i128(candidate).map(Value::Number)
            && !values_contain(existing, &value)
            && example_matches_schema(&value, schema, root, &mut BTreeSet::new())
        {
            return Some(value);
        }
    }

    None
}
