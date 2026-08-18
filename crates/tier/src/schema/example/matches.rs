use std::collections::BTreeSet;

use serde_json::Value;

use crate::schema::core::inlined_schema_ref;

mod array;
mod combinator;
mod object;
pub(in crate::schema::example) mod string;
mod ty;

use self::array::array_matches_schema;
use self::combinator::combinators_match_schema;
use self::object::object_matches_schema;
use self::string::string_matches_schema;
use self::ty::value_matches_schema_type;
use super::numeric::number_matches_numeric_schema;
use crate::value::{values_contain, values_equal};

pub(super) fn example_matches_schema(
    value: &Value,
    schema: &Value,
    root: &Value,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if !visited_refs.insert(reference.to_owned()) {
            return true;
        }

        let result = inlined_schema_ref(schema, root)
            .is_some_and(|inlined| example_matches_schema(value, &inlined, root, visited_refs));
        visited_refs.remove(reference);
        return result;
    }

    let Some(object) = schema.as_object() else {
        return schema.as_bool().unwrap_or(true);
    };

    if let Some(constant) = object.get("const")
        && !values_equal(value, constant)
    {
        return false;
    }

    if let Some(values) = object.get("enum").and_then(Value::as_array)
        && !values_contain(values, value)
    {
        return false;
    }

    if !combinators_match_schema(value, object, root, visited_refs) {
        return false;
    }

    if let Some(types) = object.get("type")
        && !value_matches_schema_type(value, types)
    {
        return false;
    }

    if let Value::Number(number) = value
        && !number_matches_numeric_schema(number, object)
    {
        return false;
    }

    match value {
        Value::String(text) => string_matches_schema(text, object),
        Value::Object(map) => object_matches_schema(map, object, root, visited_refs),
        Value::Array(items) => array_matches_schema(items, object, root, visited_refs),
        _ => true,
    }
}
