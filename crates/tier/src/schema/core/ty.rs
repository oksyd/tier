use serde_json::Value;

use crate::number::json_number_is_integer;

pub(crate) fn schema_preferred_type(object: &serde_json::Map<String, Value>) -> String {
    match object.get("type") {
        Some(Value::String(ty)) => ty.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .find(|ty| *ty != "null")
            .unwrap_or("null")
            .to_owned(),
        _ if object.contains_key("const") => object
            .get("const")
            .map_or_else(|| "object".to_owned(), infer_type_from_value),
        _ if object.contains_key("enum") => "enum".to_owned(),
        _ if object.contains_key("items") || object.contains_key("prefixItems") => {
            "array".to_owned()
        }
        _ => "object".to_owned(),
    }
}

pub(crate) fn schema_type_label(object: &serde_json::Map<String, Value>) -> String {
    match object.get("type") {
        Some(Value::String(ty)) => ty.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" | "),
        _ if object.contains_key("const") => object
            .get("const")
            .map_or_else(|| "object".to_owned(), infer_type_from_value),
        _ if object.contains_key("enum") => "enum".to_owned(),
        _ if object.contains_key("items") || object.contains_key("prefixItems") => {
            "array".to_owned()
        }
        _ => "object".to_owned(),
    }
}

fn infer_type_from_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "boolean".to_owned(),
        Value::Number(number) => {
            if json_number_is_integer(number) {
                "integer".to_owned()
            } else {
                "number".to_owned()
            }
        }
        Value::String(_) => "string".to_owned(),
        Value::Array(_) => "array".to_owned(),
        Value::Object(_) => "object".to_owned(),
    }
}
