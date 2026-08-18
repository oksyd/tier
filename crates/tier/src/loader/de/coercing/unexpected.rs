use serde::de::Unexpected;
use serde_json::Value;

pub(super) fn unexpected_value(value: &Value) -> Unexpected<'_> {
    match value {
        Value::Null => Unexpected::Unit,
        Value::Bool(value) => Unexpected::Bool(*value),
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                Unexpected::Signed(value)
            } else if let Some(value) = number.as_u64() {
                Unexpected::Unsigned(value)
            } else if let Some(value) = number.as_f64() {
                Unexpected::Float(value)
            } else {
                Unexpected::Other("number")
            }
        }
        Value::String(value) => Unexpected::Str(value),
        Value::Array(_) => Unexpected::Other("array"),
        Value::Object(_) => Unexpected::Other("object"),
    }
}
