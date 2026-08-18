use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationError;
use crate::metadata::ValidationRule;
use crate::value::values_equal;

use super::super::super::error::validation_error;

pub(in crate::loader::validation::rules) fn validate_unique_items(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    match actual {
        Value::Array(values) => has_duplicate(values).then(|| {
            validation_error(
                path,
                actual,
                rule,
                secret_paths,
                "items must be unique",
                Some(Value::Bool(true)),
            )
        }),
        _ => Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            "must be an array to apply unique-items validation",
            Some(Value::Bool(true)),
        )),
    }
}

fn has_duplicate(values: &[Value]) -> bool {
    let mut seen = Vec::<&Value>::new();
    values.iter().any(|value| {
        let duplicate = seen.iter().any(|existing| values_equal(existing, value));
        if !duplicate {
            seen.push(value);
        }
        duplicate
    })
}
