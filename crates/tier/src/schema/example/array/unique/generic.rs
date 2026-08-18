use std::collections::BTreeSet;

use serde_json::Value;

use super::super::super::matches::example_matches_schema;
use crate::value::values_contain;

pub(super) fn uniquify_generic_example_value(
    schema: &Value,
    root: &Value,
    existing: &[Value],
) -> Option<Value> {
    let mut object = serde_json::Map::new();
    object.insert("example".to_owned(), Value::Bool(true));

    let candidates = [
        Value::Null,
        Value::Bool(false),
        Value::Bool(true),
        Value::Number(0.into()),
        Value::Number(1.into()),
        Value::String("example".to_owned()),
        Value::String("value".to_owned()),
        Value::Array(Vec::new()),
        Value::Array(vec![Value::Null]),
        Value::Object(serde_json::Map::new()),
        Value::Object(object),
    ];

    candidates.into_iter().find(|candidate| {
        !values_contain(existing, candidate)
            && example_matches_schema(candidate, schema, root, &mut BTreeSet::new())
    })
}
