use std::cmp::Ordering;

use serde_json::Value;

use crate::number::compare_json_numbers;

pub(crate) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            compare_json_numbers(left, right).is_some_and(|ordering| ordering == Ordering::Equal)
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| values_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, left)| {
                    right
                        .get(key)
                        .is_some_and(|right| values_equal(left, right))
                })
        }
        _ => left == right,
    }
}

#[cfg(feature = "schema")]
pub(crate) fn values_contain(values: &[Value], value: &Value) -> bool {
    values.iter().any(|existing| values_equal(existing, value))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::values_equal;

    #[test]
    fn value_equality_treats_numbers_mathematically() {
        assert!(values_equal(&json!(1), &json!(1.0)));
        assert!(values_equal(
            &json!([1, { "port": 8080 }]),
            &json!([1.0, { "port": 8080.0 }])
        ));
        assert!(!values_equal(&json!(1), &json!(1.5)));
    }
}
