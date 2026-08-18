use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::{ConfigError, ValidationFailure, ValidationFailures};
use crate::metadata::ConfigMetadata;
use crate::report::ConfigReport;

use crate::loader::NamedValidator;
use crate::loader::validation::{
    enrich_validation_errors, validate_declared_checks, validate_declared_rules,
};

pub(super) fn validate_loaded_config<T>(
    config: &T,
    normalized_value: &Value,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
    report: &mut ConfigReport,
    validators: Vec<NamedValidator<T>>,
) -> Result<(), ConfigError> {
    let mut failures = ValidationFailures::new();
    validate_declared_metadata(
        normalized_value,
        metadata,
        secret_paths,
        report,
        &mut failures,
    );
    validate_custom_hooks(config, validators, secret_paths, report, &mut failures);

    if failures.is_empty() {
        Ok(())
    } else {
        Err(ConfigError::Validation { failures })
    }
}

fn validate_declared_metadata(
    normalized_value: &Value,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
    report: &mut ConfigReport,
    failures: &mut ValidationFailures,
) {
    let mut errors = validate_declared_rules(normalized_value, metadata, secret_paths, report);
    errors.extend(validate_declared_checks(
        normalized_value,
        metadata,
        secret_paths,
    ));
    enrich_validation_errors(&mut errors, report, secret_paths);
    if !errors.is_empty() {
        failures.push(ValidationFailure::declared(errors));
        return;
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
}

fn validate_custom_hooks<T>(
    config: &T,
    validators: Vec<NamedValidator<T>>,
    secret_paths: &BTreeSet<String>,
    report: &mut ConfigReport,
    failures: &mut ValidationFailures,
) {
    for validator in validators {
        match (validator.run)(config) {
            Ok(()) => report.record_validation(validator.name),
            Err(mut errors) => {
                enrich_validation_errors(&mut errors, report, secret_paths);
                failures.push(ValidationFailure::custom(validator.name, errors));
            }
        }
    }
}
