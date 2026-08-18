use std::collections::BTreeMap;

use serde_json::Value;

use crate::MergeStrategy;
use crate::docs::EnvDocEntry;
use crate::schema::schema_type_label as schema_type;

pub(super) fn any_leaf_doc_entry(path: &str, required: bool) -> EnvDocEntry {
    leaf_doc_entry(path, "any".to_owned(), required, false, None)
}

pub(super) fn unknown_leaf_doc_entry(path: &str, required: bool) -> EnvDocEntry {
    leaf_doc_entry(path, "unknown".to_owned(), required, false, None)
}

pub(super) fn schema_leaf_doc_entry(
    path: &str,
    required: bool,
    object: &serde_json::Map<String, Value>,
) -> EnvDocEntry {
    leaf_doc_entry(
        path,
        schema_type(object),
        required,
        schema_node_is_secret(object),
        object
            .get("description")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    )
}

fn leaf_doc_entry(
    path: &str,
    ty: String,
    required: bool,
    secret: bool,
    description: Option<String>,
) -> EnvDocEntry {
    EnvDocEntry {
        path: path.to_owned(),
        env: String::new(),
        ty,
        required,
        secret,
        description,
        example: None,
        deprecated: None,
        aliases: Vec::new(),
        has_default: false,
        merge: MergeStrategy::Merge,
        allowed_sources: Vec::new(),
        denied_sources: Vec::new(),
        validations: Vec::new(),
        validation_levels: BTreeMap::new(),
        validation_messages: BTreeMap::new(),
        validation_tags: BTreeMap::new(),
    }
}

fn schema_node_is_secret(object: &serde_json::Map<String, Value>) -> bool {
    object
        .get("writeOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || object
            .get("x-tier-secret")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}
