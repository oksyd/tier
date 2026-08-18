use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationError;
use crate::metadata::ValidationRule;

use super::super::super::error::validation_error;

pub(in crate::loader::validation::rules) fn validate_non_empty(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    let is_empty = match actual {
        Value::String(value) => value.is_empty(),
        Value::Array(values) => values.is_empty(),
        Value::Object(values) => values.is_empty(),
        _ => false,
    };
    is_empty.then(|| validation_error(path, actual, rule, secret_paths, "must not be empty", None))
}
