use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationError;
use crate::metadata::ValidationRule;

use super::super::super::error::validation_error;

pub(in crate::loader::validation::rules) fn validate_min_length(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    min: usize,
) -> Option<ValidationError> {
    validate_count_bound(
        Bound::Min,
        CountValidation {
            path,
            actual,
            rule,
            secret_paths,
            observed: validation_length(actual),
            limit: min,
            label: "length",
            type_error: "must be a string, array, or object to apply length validation",
        },
    )
}

pub(in crate::loader::validation::rules) fn validate_max_length(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    max: usize,
) -> Option<ValidationError> {
    validate_count_bound(
        Bound::Max,
        CountValidation {
            path,
            actual,
            rule,
            secret_paths,
            observed: validation_length(actual),
            limit: max,
            label: "length",
            type_error: "must be a string, array, or object to apply length validation",
        },
    )
}

pub(in crate::loader::validation::rules) fn validate_min_items(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    min: usize,
) -> Option<ValidationError> {
    validate_count_bound(
        Bound::Min,
        CountValidation {
            path,
            actual,
            rule,
            secret_paths,
            observed: array_len(actual),
            limit: min,
            label: "item count",
            type_error: "must be an array to apply item-count validation",
        },
    )
}

pub(in crate::loader::validation::rules) fn validate_max_items(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    max: usize,
) -> Option<ValidationError> {
    validate_count_bound(
        Bound::Max,
        CountValidation {
            path,
            actual,
            rule,
            secret_paths,
            observed: array_len(actual),
            limit: max,
            label: "item count",
            type_error: "must be an array to apply item-count validation",
        },
    )
}

pub(in crate::loader::validation::rules) fn validate_min_properties(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    min: usize,
) -> Option<ValidationError> {
    validate_count_bound(
        Bound::Min,
        CountValidation {
            path,
            actual,
            rule,
            secret_paths,
            observed: object_len(actual),
            limit: min,
            label: "property count",
            type_error: "must be an object to apply property-count validation",
        },
    )
}

pub(in crate::loader::validation::rules) fn validate_max_properties(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    max: usize,
) -> Option<ValidationError> {
    validate_count_bound(
        Bound::Max,
        CountValidation {
            path,
            actual,
            rule,
            secret_paths,
            observed: object_len(actual),
            limit: max,
            label: "property count",
            type_error: "must be an object to apply property-count validation",
        },
    )
}

#[derive(Clone, Copy)]
enum Bound {
    Min,
    Max,
}

impl Bound {
    fn accepts(self, observed: usize, limit: usize) -> bool {
        match self {
            Self::Min => observed >= limit,
            Self::Max => observed <= limit,
        }
    }

    fn operator(self) -> &'static str {
        match self {
            Self::Min => ">=",
            Self::Max => "<=",
        }
    }
}

struct CountValidation<'a> {
    path: &'a str,
    actual: &'a Value,
    rule: &'a ValidationRule,
    secret_paths: &'a BTreeSet<String>,
    observed: Option<usize>,
    limit: usize,
    label: &'static str,
    type_error: &'static str,
}

fn validate_count_bound(bound: Bound, validation: CountValidation<'_>) -> Option<ValidationError> {
    let expected = || {
        Some(Value::Number(
            u64::try_from(validation.limit).unwrap_or(u64::MAX).into(),
        ))
    };
    match validation.observed {
        Some(observed) if bound.accepts(observed, validation.limit) => None,
        Some(_) => Some(validation_error(
            validation.path,
            validation.actual,
            validation.rule,
            validation.secret_paths,
            &format!(
                "{} must be {} {}",
                validation.label,
                bound.operator(),
                validation.limit
            ),
            expected(),
        )),
        None => Some(validation_error(
            validation.path,
            validation.actual,
            validation.rule,
            validation.secret_paths,
            validation.type_error,
            expected(),
        )),
    }
}

fn validation_length(value: &Value) -> Option<usize> {
    match value {
        Value::String(inner) => Some(inner.chars().count()),
        Value::Array(values) => Some(values.len()),
        Value::Object(values) => Some(values.len()),
        _ => None,
    }
}

fn array_len(value: &Value) -> Option<usize> {
    match value {
        Value::Array(values) => Some(values.len()),
        _ => None,
    }
}

fn object_len(value: &Value) -> Option<usize> {
    match value {
        Value::Object(values) => Some(values.len()),
        _ => None,
    }
}
