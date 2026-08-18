use serde_json::{Number, Value};

use crate::number::{compare_json_numbers, json_number_is_multiple_of};

mod bounds;
mod integer;
mod real;

pub(super) use self::integer::{
    fallback_integer_example, first_integer_multiple_at_or_above, integer_multiple_step_constraint,
    last_integer_multiple_at_or_below,
};
pub(super) use self::real::fallback_number_example;

pub(super) fn number_matches_numeric_schema(
    number: &Number,
    object: &serde_json::Map<String, Value>,
) -> bool {
    if let Some(minimum) = object.get("minimum").and_then(Value::as_number)
        && compare_json_numbers(number, minimum).is_some_and(|ordering| ordering.is_lt())
    {
        return false;
    }
    if let Some(maximum) = object.get("maximum").and_then(Value::as_number)
        && compare_json_numbers(number, maximum).is_some_and(|ordering| ordering.is_gt())
    {
        return false;
    }
    if let Some(minimum) = object.get("exclusiveMinimum").and_then(Value::as_number)
        && compare_json_numbers(number, minimum).is_some_and(|ordering| !ordering.is_gt())
    {
        return false;
    }
    if let Some(maximum) = object.get("exclusiveMaximum").and_then(Value::as_number)
        && compare_json_numbers(number, maximum).is_some_and(|ordering| !ordering.is_lt())
    {
        return false;
    }

    number_matches_multiple_of(number, object)
}

fn number_matches_multiple_of(number: &Number, object: &serde_json::Map<String, Value>) -> bool {
    let Some(multiple_of) = object.get("multipleOf").and_then(Value::as_number) else {
        return true;
    };

    json_number_is_multiple_of(number, multiple_of).unwrap_or(true)
}
