use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::error::ConfigError;
use crate::metadata::ConfigMetadata;
use crate::report::ConfigReport;

use crate::loader::NamedValidator;
use crate::loader::canonical::canonicalize_value_paths;
use crate::loader::validation::{validate_declared_checks, validate_declared_rules};

pub(super) fn validate_loaded_config<T>(
    config: &T,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
    report: &mut ConfigReport,
    validators: Vec<NamedValidator<T>>,
) -> Result<Value, ConfigError>
where
    T: Serialize,
{
    let normalized_value = canonicalize_value_paths(&serde_json::to_value(config)?, metadata)?;
    validate_declared_metadata(&normalized_value, metadata, secret_paths, report)?;
    validate_custom_hooks(config, validators, report)?;
    Ok(normalized_value)
}

fn validate_declared_metadata(
    normalized_value: &Value,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
    report: &mut ConfigReport,
) -> Result<(), ConfigError> {
    let mut declared_errors =
        validate_declared_rules(normalized_value, metadata, secret_paths, report);
    declared_errors.extend(validate_declared_checks(
        normalized_value,
        metadata,
        secret_paths,
    ));
    if !declared_errors.is_empty() {
        return Err(ConfigError::DeclaredValidation {
            errors: declared_errors,
        });
    }

    if metadata
        .fields()
        .iter()
        .any(|field| !field.validations.is_empty())
    {
        report.record_validation("tier::declared.fields".to_owned());
    }
    if !metadata.checks().is_empty() {
        report.record_validation("tier::declared.checks".to_owned());
    }

    Ok(())
}

fn validate_custom_hooks<T>(
    config: &T,
    validators: Vec<NamedValidator<T>>,
    report: &mut ConfigReport,
) -> Result<(), ConfigError> {
    for validator in validators {
        (validator.run)(config).map_err(|errors| ConfigError::Validation {
            name: validator.name.clone(),
            errors,
        })?;
        report.record_validation(validator.name);
    }

    Ok(())
}
