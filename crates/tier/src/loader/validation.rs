use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationErrors;
use crate::metadata::ConfigMetadata;
use crate::path::get_value_at_path;
use crate::report::{ConfigReport, ConfigWarning};

mod checks;
mod error;
mod matching;
mod rules;

use self::matching::{collect_matching_values, is_present_value};
use self::rules::validate_declared_rule;

pub(super) fn validate_declared_rules(
    value: &Value,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
    report: &mut ConfigReport,
) -> ValidationErrors {
    let mut errors = ValidationErrors::new();
    let mut matched_paths = BTreeSet::<String>::new();

    for field in metadata.fields() {
        if field.validations.is_empty() || field.path.is_empty() {
            continue;
        }
        matched_paths.extend(
            collect_matching_values(value, &field.path)
                .into_iter()
                .map(|(matched_path, _)| matched_path),
        );
    }

    for matched_path in matched_paths {
        let Some(actual) = get_value_at_path(value, &matched_path) else {
            continue;
        };
        if !is_present_value(actual) {
            continue;
        }
        for effective in metadata.effective_validations_for(&matched_path) {
            let rule = &effective.rule;
            let field = effective.field;
            if let Some(error) = validate_declared_rule(&matched_path, actual, rule, secret_paths) {
                let error = field.decorate_validation_error(rule, error);
                match field.validation_level_for(rule) {
                    crate::ValidationLevel::Error => errors.push(error),
                    crate::ValidationLevel::Warning => {
                        report.record_warning(ConfigWarning::Validation(error));
                    }
                }
            }
        }
    }

    errors
}

pub(super) fn validate_declared_checks(
    value: &Value,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
) -> ValidationErrors {
    checks::validate_declared_checks(value, metadata, secret_paths)
}
