use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde_json::{Number, Value};

use crate::error::ValidationError;
use crate::metadata::{ValidationNumber, ValidationRule};
use crate::number::{compare_json_numbers, json_number_is_multiple_of};

use super::super::error::validation_error;

pub(super) fn validate_min(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    min: &ValidationNumber,
) -> Option<ValidationError> {
    validate_numeric_bound(NumericBound::Min, path, actual, rule, secret_paths, min)
}

pub(super) fn validate_max(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    max: &ValidationNumber,
) -> Option<ValidationError> {
    validate_numeric_bound(NumericBound::Max, path, actual, rule, secret_paths, max)
}

pub(super) fn validate_multiple_of(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    factor: &ValidationNumber,
) -> Option<ValidationError> {
    let Some(divisor) = factor.as_f64().filter(|value| value.is_finite()) else {
        return Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            &format!("declared multiple_of factor must be finite, got {factor}"),
            Some(factor.as_json_value()),
        ));
    };
    if divisor <= 0.0 {
        return Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            "declared multiple_of factor must be > 0",
            Some(factor.as_json_value()),
        ));
    }

    match actual.as_number() {
        Some(value) if number_is_multiple_of(value, factor) => None,
        Some(_) => Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            &format!("must be a multiple of {factor}"),
            Some(factor.as_json_value()),
        )),
        None => Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            "must be a numeric value",
            Some(factor.as_json_value()),
        )),
    }
}

fn compare_value_to_bound(value: &Value, bound: &ValidationNumber) -> Option<Ordering> {
    compare_json_numbers(value.as_number()?, validation_number(bound)?)
}

#[derive(Clone, Copy)]
enum NumericBound {
    Min,
    Max,
}

impl NumericBound {
    fn accepts(self, ordering: Ordering) -> bool {
        match self {
            Self::Min => matches!(ordering, Ordering::Greater | Ordering::Equal),
            Self::Max => matches!(ordering, Ordering::Less | Ordering::Equal),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Min => "minimum",
            Self::Max => "maximum",
        }
    }

    fn operator(self) -> &'static str {
        match self {
            Self::Min => ">=",
            Self::Max => "<=",
        }
    }
}

fn validate_numeric_bound(
    bound: NumericBound,
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    limit: &ValidationNumber,
) -> Option<ValidationError> {
    if !limit.is_finite() {
        return Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            &format!("declared {} must be finite, got {limit}", bound.name()),
            Some(limit.as_json_value()),
        ));
    }

    match compare_value_to_bound(actual, limit) {
        Some(ordering) if bound.accepts(ordering) => None,
        Some(_) => Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            &format!("must be {} {limit}", bound.operator()),
            Some(limit.as_json_value()),
        )),
        None => Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            "must be a numeric value",
            Some(limit.as_json_value()),
        )),
    }
}

fn number_is_multiple_of(value: &Number, factor: &ValidationNumber) -> bool {
    let Some(factor_number) = validation_number(factor) else {
        return false;
    };

    json_number_is_multiple_of(value, factor_number).unwrap_or(false)
}

fn validation_number(value: &ValidationNumber) -> Option<&Number> {
    match value {
        ValidationNumber::Finite(value) => Some(value),
        ValidationNumber::Invalid(_) => None,
    }
}
