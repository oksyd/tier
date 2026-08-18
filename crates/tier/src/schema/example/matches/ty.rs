use serde_json::Value;

use crate::number::json_number_is_integer;

pub(super) fn value_matches_schema_type(value: &Value, types: &Value) -> bool {
    match types {
        Value::String(ty) => value_matches_single_schema_type(value, ty),
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .any(|ty| value_matches_single_schema_type(value, ty)),
        _ => true,
    }
}

fn value_matches_single_schema_type(value: &Value, ty: &str) -> bool {
    match ty {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "string" => value.is_string(),
        "integer" => value.as_number().is_some_and(json_number_is_integer),
        "number" => value.is_number(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => true,
    }
}
