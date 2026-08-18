use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationError;
use crate::metadata::{ValidationCheck, ValidationRule};
use crate::path::{join_path, normalize_path};

use super::super::is_secret_path;

pub(super) fn validation_error(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    message: &str,
    expected: Option<Value>,
) -> ValidationError {
    let actual = if is_secret_path(secret_paths, path) {
        Value::String("***redacted***".to_owned())
    } else {
        actual.clone()
    };

    let mut error = ValidationError::new(path, message).with_rule(rule.code());
    if let Some(expected) = expected {
        error = error.with_expected(expected);
    }
    error.with_actual(actual)
}

pub(super) fn group_validation_error<I, S>(
    check: &ValidationCheck,
    related_paths: I,
    secret_paths: &BTreeSet<String>,
    message: &str,
    expected: Option<Value>,
    actual: Option<Value>,
) -> ValidationError
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let related_paths = related_paths
        .into_iter()
        .map(|path| normalize_path(path.as_ref()))
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();

    let actual = actual.map(|value| redact_group_value(value, &related_paths, secret_paths));

    let mut error = ValidationError::new("", message)
        .with_rule(check.code())
        .with_related_paths(related_paths);
    if let Some(expected) = expected {
        error = error.with_expected(expected);
    }
    if let Some(actual) = actual {
        error = error.with_actual(actual);
    }
    error
}

fn redact_group_value(
    value: Value,
    related_paths: &[String],
    secret_paths: &BTreeSet<String>,
) -> Value {
    let mut value = value;
    redact_group_value_recursive(&mut value, "", related_paths, secret_paths);
    value
}

fn redact_group_value_recursive(
    value: &mut Value,
    current: &str,
    related_paths: &[String],
    secret_paths: &BTreeSet<String>,
) {
    if related_paths.iter().any(|path| path == current) && is_secret_path(secret_paths, current) {
        *value = Value::String("***redacted***".to_owned());
        return;
    }

    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = join_path(current, key);
                redact_group_value_recursive(child, &next, related_paths, secret_paths);
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter_mut().enumerate() {
                let next = join_path(current, &index.to_string());
                redact_group_value_recursive(child, &next, related_paths, secret_paths);
            }
        }
        _ => {}
    }
}
