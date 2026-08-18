use std::collections::BTreeSet;

use crate::ConfigError;
use crate::metadata::paths::validate_metadata_path;
use crate::metadata::{ConfigMetadata, FieldMetadata, ValidationRule};

pub(super) fn validate_field(
    metadata: &ConfigMetadata,
    field: &FieldMetadata,
) -> Result<(), ConfigError> {
    validate_metadata_path(&field.path)?;
    validate_root_restrictions(field)?;
    validate_source_policy(field)?;
    validate_validation_configs(metadata, field)?;
    validate_aliases(field)
}

fn validate_aliases(field: &FieldMetadata) -> Result<(), ConfigError> {
    for alias in &field.aliases {
        validate_metadata_path(alias)?;
        if alias.is_empty() {
            return Err(ConfigError::MetadataInvalid {
                path: alias.clone(),
                message: "aliases cannot target the root path".to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_root_restrictions(field: &FieldMetadata) -> Result<(), ConfigError> {
    if field.path.is_empty() && !field.aliases.is_empty() {
        let alias = field.aliases.first().cloned().unwrap_or_default();
        return Err(ConfigError::MetadataInvalid {
            path: alias,
            message: "aliases cannot rewrite the root path".to_owned(),
        });
    }
    if field.path.is_empty() && field.merge_explicit {
        return Err(root_error(
            field,
            "merge strategies cannot target the root path",
        ));
    }
    if field.path.is_empty() && field.allowed_sources.is_some() {
        return Err(root_error(
            field,
            "source policies cannot target the root path",
        ));
    }
    if field.path.is_empty() && field.denied_sources.is_some() {
        return Err(root_error(
            field,
            "source policies cannot target the root path",
        ));
    }
    if field.path.is_empty() && !field.validations.is_empty() {
        return Err(root_error(
            field,
            "validation rules cannot target the root path",
        ));
    }
    if field.path.is_empty() && !field.validation_configs.is_empty() {
        return Err(root_error(
            field,
            "validation rules cannot target the root path",
        ));
    }
    if field.path.is_empty() && field.secret {
        return Err(root_error(
            field,
            "secret metadata cannot target the root path",
        ));
    }
    if field.path.is_empty() && field.deprecated.is_some() {
        return Err(root_error(
            field,
            "deprecation metadata cannot target the root path",
        ));
    }
    if field.path.is_empty() && field.env_decode.is_some() {
        return Err(root_error(
            field,
            "environment decoder paths cannot target the root path",
        ));
    }
    Ok(())
}

fn root_error(field: &FieldMetadata, message: &str) -> ConfigError {
    ConfigError::MetadataInvalid {
        path: field.path.clone(),
        message: message.to_owned(),
    }
}

fn validate_source_policy(field: &FieldMetadata) -> Result<(), ConfigError> {
    if let Some(allowed_sources) = &field.allowed_sources
        && allowed_sources.is_empty()
    {
        return Err(ConfigError::MetadataInvalid {
            path: field.path.clone(),
            message: "source policies must allow at least one source kind".to_owned(),
        });
    }
    if let Some(denied_sources) = &field.denied_sources
        && let Some(allowed_sources) = &field.allowed_sources
    {
        let overlap = allowed_sources
            .intersection(denied_sources)
            .copied()
            .collect::<Vec<_>>();
        if !overlap.is_empty() {
            return Err(ConfigError::MetadataInvalid {
                path: field.path.clone(),
                message: format!(
                    "source policies cannot both allow and deny the same source kinds: {}",
                    overlap
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }
    }

    Ok(())
}

fn validate_validation_configs(
    metadata: &ConfigMetadata,
    field: &FieldMetadata,
) -> Result<(), ConfigError> {
    let effective_rule_codes = metadata
        .effective_field_for(&field.path)
        .map(|field| {
            field
                .validations
                .iter()
                .map(ValidationRule::code)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    for rule_code in field.validation_configs.keys() {
        if !effective_rule_codes.contains(rule_code.as_str()) {
            return Err(ConfigError::MetadataInvalid {
                path: field.path.clone(),
                message: format!(
                    "validation config references unknown rule `{rule_code}` for this field"
                ),
            });
        }
    }

    Ok(())
}
