use serde_json::Value;

use crate::{FieldMetadata, ValidationLevel};

use super::redaction::{is_secret_schema_node, redact_example_value};
use super::validation::apply_validation_schema_keywords;

pub(super) fn annotate_schema_node(node: &mut Value, field: &FieldMetadata) {
    let Some(object) = node.as_object_mut() else {
        return;
    };
    let is_secret = field.secret || is_secret_schema_node(object);

    if let Some(doc) = &field.doc {
        object.insert("description".to_owned(), Value::String(doc.clone()));
    }
    if let Some(example) = &field.example {
        let value = parse_example_value(example);
        object.insert(
            "example".to_owned(),
            if is_secret {
                redact_example_value(&value)
            } else {
                value
            },
        );
    }
    if let Some(note) = &field.deprecated {
        object.insert("deprecated".to_owned(), Value::Bool(true));
        object.insert(
            "x-tier-deprecated-note".to_owned(),
            Value::String(note.clone()),
        );
    }
    if is_secret {
        object.insert("writeOnly".to_owned(), Value::Bool(true));
        object.insert("x-tier-secret".to_owned(), Value::Bool(true));
    }
    if let Some(env) = &field.env {
        object.insert("x-tier-env".to_owned(), Value::String(env.clone()));
    }
    if let Some(env_decode) = &field.env_decode {
        object.insert(
            "x-tier-env-decode".to_owned(),
            Value::String(env_decode.to_string()),
        );
    }
    if !field.aliases.is_empty() {
        object.insert(
            "x-tier-aliases".to_owned(),
            Value::Array(field.aliases.iter().cloned().map(Value::String).collect()),
        );
    }
    if field.has_default {
        object.insert("x-tier-has-default".to_owned(), Value::Bool(true));
    }
    object.insert(
        "x-tier-merge".to_owned(),
        Value::String(field.merge.to_string()),
    );

    insert_source_policy_annotations(node, field);
    insert_validation_annotations(node, field);
}

fn insert_source_policy_annotations(node: &mut Value, field: &FieldMetadata) {
    let Some(object) = node.as_object_mut() else {
        return;
    };

    let allowed_sources = field.allowed_source_names();
    if !allowed_sources.is_empty() {
        object.insert(
            "x-tier-sources".to_owned(),
            Value::Array(allowed_sources.into_iter().map(Value::String).collect()),
        );
    }
    let denied_sources = field.denied_source_names();
    if !denied_sources.is_empty() {
        object.insert(
            "x-tier-denied-sources".to_owned(),
            Value::Array(denied_sources.into_iter().map(Value::String).collect()),
        );
    }
}

fn insert_validation_annotations(node: &mut Value, field: &FieldMetadata) {
    let Some(object) = node.as_object_mut() else {
        return;
    };

    let schema_validations = field
        .validations
        .iter()
        .filter(|rule| field.validation_level_for(rule) == ValidationLevel::Error)
        .cloned()
        .collect::<Vec<_>>();
    apply_validation_schema_keywords(object, &schema_validations);
    if !field.validations.is_empty() {
        object.insert(
            "x-tier-validate".to_owned(),
            serde_json::to_value(&field.validations).unwrap_or_else(|_| Value::Array(Vec::new())),
        );
    }
    if let Some(validation_config) = field.validation_config_json() {
        object.insert("x-tier-validation-config".to_owned(), validation_config);
    }
}

fn parse_example_value(example: &str) -> Value {
    serde_json::from_str(example).unwrap_or_else(|_| Value::String(example.to_owned()))
}
