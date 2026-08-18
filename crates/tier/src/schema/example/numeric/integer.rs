use serde_json::{Number, Value};

use super::number_matches_numeric_schema;
use crate::number::{
    finite_f64_to_i128, integer_multiple_step_from_json_number, json_number_as_i128,
    json_number_from_i128,
};

pub(in crate::schema::example) fn fallback_integer_example(
    object: &serde_json::Map<String, Value>,
    accepts: impl Fn(&Number) -> bool,
) -> Option<Number> {
    let lower = integer_lower_bound(object).unwrap_or(0);
    let upper = integer_upper_bound(object);
    let accepts = &accepts;
    let candidate = integer_candidate_in_range(lower, upper, object, accepts)
        .or_else(|| {
            upper.filter(|upper| integer_value_matches_constraints(*upper, object, accepts))
        })
        .or_else(|| integer_value_matches_constraints(lower, object, accepts).then_some(lower))
        .or_else(|| integer_value_matches_constraints(0, object, accepts).then_some(0))?;

    json_number_from_i128(candidate)
}

pub(in crate::schema::example) fn integer_multiple_step_constraint(
    object: &serde_json::Map<String, Value>,
) -> Option<i128> {
    object
        .get("multipleOf")
        .and_then(Value::as_number)
        .and_then(integer_multiple_step_from_json_number)
}

pub(in crate::schema::example) fn first_integer_multiple_at_or_above(
    lower: i128,
    factor: i128,
) -> Option<i128> {
    if factor <= 0 {
        return None;
    }

    let quotient = lower.div_euclid(factor);
    let candidate = quotient.checked_mul(factor)?;
    if candidate >= lower {
        Some(candidate)
    } else {
        candidate.checked_add(factor)
    }
}

pub(in crate::schema::example) fn last_integer_multiple_at_or_below(
    upper: i128,
    factor: i128,
) -> Option<i128> {
    if factor <= 0 {
        return None;
    }

    upper.div_euclid(factor).checked_mul(factor)
}

fn integer_candidate_in_range(
    lower: i128,
    upper: Option<i128>,
    object: &serde_json::Map<String, Value>,
    accepts: &impl Fn(&Number) -> bool,
) -> Option<i128> {
    if let Some(step) = integer_multiple_step_constraint(object)
        && let Some(candidate) = first_integer_multiple_at_or_above(lower, step)
        && upper.is_none_or(|upper| candidate <= upper)
        && integer_value_matches_constraints(candidate, object, accepts)
    {
        return Some(candidate);
    }

    let mut candidate = lower;
    let max_iterations = upper
        .and_then(|upper| upper.checked_sub(lower))
        .and_then(|span| span.checked_add(1))
        .and_then(|span| usize::try_from(span).ok())
        .unwrap_or(10_000);

    for _ in 0..max_iterations {
        if upper.is_some_and(|upper| candidate > upper) {
            break;
        }
        if integer_value_matches_constraints(candidate, object, accepts) {
            return Some(candidate);
        }
        candidate = candidate.saturating_add(1);
    }

    None
}

fn integer_lower_bound(object: &serde_json::Map<String, Value>) -> Option<i128> {
    let inclusive = object.get("minimum").and_then(schema_number_ceil);
    let exclusive = object
        .get("exclusiveMinimum")
        .and_then(schema_number_floor)
        .and_then(|minimum| minimum.checked_add(1));

    max_bound(inclusive, exclusive)
}

fn integer_upper_bound(object: &serde_json::Map<String, Value>) -> Option<i128> {
    let inclusive = object.get("maximum").and_then(schema_number_floor);
    let exclusive = object
        .get("exclusiveMaximum")
        .and_then(schema_number_ceil)
        .and_then(|maximum| maximum.checked_sub(1));

    min_bound(inclusive, exclusive)
}

fn schema_number_floor(value: &Value) -> Option<i128> {
    let number = value.as_number()?;
    json_number_as_i128(number).or_else(|| finite_f64_to_i128(number.as_f64()?.floor()))
}

fn schema_number_ceil(value: &Value) -> Option<i128> {
    let number = value.as_number()?;
    json_number_as_i128(number).or_else(|| finite_f64_to_i128(number.as_f64()?.ceil()))
}

fn integer_value_matches_constraints(
    value: i128,
    object: &serde_json::Map<String, Value>,
    accepts: &impl Fn(&Number) -> bool,
) -> bool {
    json_number_from_i128(value)
        .is_some_and(|number| number_matches_numeric_schema(&number, object) && accepts(&number))
}

fn max_bound(left: Option<i128>, right: Option<i128>) -> Option<i128> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(bound), None) | (None, Some(bound)) => Some(bound),
        (None, None) => None,
    }
}

fn min_bound(left: Option<i128>, right: Option<i128>) -> Option<i128> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(bound), None) | (None, Some(bound)) => Some(bound),
        (None, None) => None,
    }
}
