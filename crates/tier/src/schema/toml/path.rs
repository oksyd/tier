use crate::ConfigMetadata;
use crate::path::{join_path, path_is_at_or_below};

pub(super) fn resolve_toml_array_item_metadata_path(
    parent_path: &str,
    index: usize,
    metadata: &ConfigMetadata,
) -> String {
    let concrete = join_metadata_path(parent_path, &index.to_string());
    if metadata_has_path_or_descendant(metadata, &concrete) {
        concrete
    } else {
        join_metadata_path(parent_path, "*")
    }
}

pub(super) fn metadata_has_path_or_descendant(metadata: &ConfigMetadata, path: &str) -> bool {
    metadata
        .fields()
        .iter()
        .any(|field| path_is_at_or_below(&field.path, path))
}

pub(super) fn join_metadata_path(parent: &str, segment: &str) -> String {
    join_path(parent, segment)
}

pub(super) fn resolve_toml_object_child_metadata_path(
    parent_path: &str,
    key: &str,
    metadata: &ConfigMetadata,
) -> String {
    let literal = join_metadata_path(parent_path, key);
    if metadata_has_path_or_descendant(metadata, &literal) {
        return literal;
    }

    if !is_dynamic_placeholder_key(key) {
        return literal;
    }

    let wildcard = join_metadata_path(parent_path, "*");
    if metadata_has_path_or_descendant(metadata, &wildcard) {
        wildcard
    } else {
        literal
    }
}

fn is_dynamic_placeholder_key(key: &str) -> bool {
    key.starts_with('{') && key.ends_with('}') && key.len() > 2
}
