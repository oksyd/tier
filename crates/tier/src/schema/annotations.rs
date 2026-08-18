mod field;
mod redaction;
mod traversal;
mod validation;

use serde_json::Value;

use crate::ConfigMetadata;

use self::traversal::annotate_schema_path;

pub(super) use self::redaction::{
    is_secret_schema_node, redact_example_value, redact_secret_schema_examples,
};

pub(super) fn apply_metadata_annotations(schema: &mut Value, metadata: &ConfigMetadata) {
    let snapshot = schema.clone();
    apply_root_checks(schema, metadata);

    for field in metadata.fields() {
        let effective = metadata
            .effective_field_for_path_with_intent(&field.path, &field.path_explicit_array_segments)
            .unwrap_or_else(|| field.clone());
        annotate_schema_path(
            schema,
            &snapshot,
            &field.path,
            &field.path_explicit_array_segments,
            &effective,
        );
    }
}

fn apply_root_checks(schema: &mut Value, metadata: &ConfigMetadata) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    if metadata.checks().is_empty() {
        return;
    }

    object.insert(
        "x-tier-checks".to_owned(),
        serde_json::to_value(metadata.checks()).unwrap_or_else(|_| Value::Array(Vec::new())),
    );
}
