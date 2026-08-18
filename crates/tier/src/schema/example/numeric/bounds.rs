use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub(super) struct NumericBound {
    pub(super) value: f64,
    pub(super) exclusive: bool,
}

pub(super) fn lower_bound(object: &serde_json::Map<String, Value>) -> Option<NumericBound> {
    stricter_lower_bound(
        finite_bound(object, "minimum", false),
        finite_bound(object, "exclusiveMinimum", true),
    )
}

pub(super) fn upper_bound(object: &serde_json::Map<String, Value>) -> Option<NumericBound> {
    stricter_upper_bound(
        finite_bound(object, "maximum", false),
        finite_bound(object, "exclusiveMaximum", true),
    )
}

fn finite_bound(
    object: &serde_json::Map<String, Value>,
    key: &str,
    exclusive: bool,
) -> Option<NumericBound> {
    object
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| NumericBound { value, exclusive })
}

fn stricter_lower_bound(
    left: Option<NumericBound>,
    right: Option<NumericBound>,
) -> Option<NumericBound> {
    match (left, right) {
        (Some(left), Some(right)) => {
            if right.value > left.value || (right.value == left.value && right.exclusive) {
                Some(right)
            } else {
                Some(left)
            }
        }
        (Some(bound), None) | (None, Some(bound)) => Some(bound),
        (None, None) => None,
    }
}

fn stricter_upper_bound(
    left: Option<NumericBound>,
    right: Option<NumericBound>,
) -> Option<NumericBound> {
    match (left, right) {
        (Some(left), Some(right)) => {
            if right.value < left.value || (right.value == left.value && right.exclusive) {
                Some(right)
            } else {
                Some(left)
            }
        }
        (Some(bound), None) | (None, Some(bound)) => Some(bound),
        (None, None) => None,
    }
}
