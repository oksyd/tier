use std::collections::BTreeSet;

use serde_json::Value;

use crate::path::join_path;

pub(in crate::loader) fn collect_string_leaf_suffixes(
    value: &Value,
    prefix: &str,
) -> BTreeSet<String> {
    let mut suffixes = BTreeSet::new();
    collect_string_leaf_suffixes_inner(value, prefix, &mut suffixes);
    suffixes
}

fn collect_string_leaf_suffixes_inner(
    value: &Value,
    prefix: &str,
    suffixes: &mut BTreeSet<String>,
) {
    match value {
        Value::String(_) => {
            suffixes.insert(prefix.to_owned());
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                let next = join_path(prefix, &index.to_string());
                collect_string_leaf_suffixes_inner(value, &next, suffixes);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                let next = join_path(prefix, key);
                collect_string_leaf_suffixes_inner(value, &next, suffixes);
            }
        }
        _ => {}
    }
}
