use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationError;
use crate::metadata::{ValidationRule, ValidationValue};
use crate::value::values_equal;

use super::super::super::error::validation_error;

pub(in crate::loader::validation::rules) fn validate_one_of(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    values: &[ValidationValue],
) -> Option<ValidationError> {
    let expected = Value::Array(values.iter().map(|value| value.0.clone()).collect());
    if values.iter().any(|value| values_equal(&value.0, actual)) {
        return None;
    }

    let sensitive = secret_paths
        .iter()
        .any(|secret| crate::path::path_overlaps_pattern(path, secret));
    let message = if sensitive {
        "must be one of the configured allowed values".to_owned()
    } else {
        format!(
            "must be one of {}",
            values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    Some(validation_error(
        path,
        actual,
        rule,
        secret_paths,
        &message,
        Some(expected),
    ))
}
