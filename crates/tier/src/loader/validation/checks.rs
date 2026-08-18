use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationErrors;
use crate::metadata::{ConfigMetadata, ValidationCheckSpec};
use crate::value::values_equal;

use super::error::group_validation_error;
use super::matching::{
    bind_required_paths, collect_matching_values, is_present_value, missing_paths, present_paths,
};

pub(super) fn validate_declared_checks(
    value: &Value,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
) -> ValidationErrors {
    let mut errors = ValidationErrors::new();

    for check in metadata.check_specs() {
        match check {
            ValidationCheckSpec::AtLeastOneOf { paths } => {
                let paths = public_paths(paths);
                let present = present_paths(value, &paths);
                if present.is_empty() {
                    let public_check = check.to_public();
                    errors.push(group_validation_error(
                        &public_check,
                        paths.clone(),
                        secret_paths,
                        &format!("at least one of {} must be configured", paths.join(", ")),
                        Some(serde_json::json!({ "min_present": 1, "paths": paths })),
                        Some(serde_json::json!({ "present": present })),
                    ));
                }
            }
            ValidationCheckSpec::ExactlyOneOf { paths } => {
                let paths = public_paths(paths);
                let present = present_paths(value, &paths);
                if present.len() != 1 {
                    let public_check = check.to_public();
                    errors.push(group_validation_error(
                        &public_check,
                        paths.clone(),
                        secret_paths,
                        &format!("exactly one of {} must be configured", paths.join(", ")),
                        Some(serde_json::json!({ "exactly_one_of": paths })),
                        Some(serde_json::json!({ "present": present })),
                    ));
                }
            }
            ValidationCheckSpec::MutuallyExclusive { paths } => {
                let paths = public_paths(paths);
                let present = present_paths(value, &paths);
                if present.len() > 1 {
                    let public_check = check.to_public();
                    errors.push(group_validation_error(
                        &public_check,
                        paths.clone(),
                        secret_paths,
                        &format!("{} are mutually exclusive", paths.join(", ")),
                        Some(serde_json::json!({ "max_present": 1, "paths": paths })),
                        Some(serde_json::json!({ "present": present })),
                    ));
                }
            }
            ValidationCheckSpec::RequiredWith { path, requires } => {
                let path = path.path.clone();
                let requires = public_paths(requires);
                for (matched_path, _) in collect_matching_values(value, &path)
                    .into_iter()
                    .filter(|(_, matched)| is_present_value(matched))
                {
                    let bound_requires = bind_required_paths(&path, &matched_path, &requires)
                        .unwrap_or_else(|| requires.clone());
                    let missing = missing_paths(value, &bound_requires);
                    if !missing.is_empty() {
                        let public_check = check.to_public();
                        errors.push(group_validation_error(
                            &public_check,
                            std::iter::once(matched_path.clone()).chain(missing.iter().cloned()),
                            secret_paths,
                            &format!("{matched_path} requires {}", missing.join(", ")),
                            Some(serde_json::json!({
                                "trigger": matched_path,
                                "requires": bound_requires
                            })),
                            Some(serde_json::json!({ "missing": missing })),
                        ));
                    }
                }
            }
            ValidationCheckSpec::RequiredIf {
                path,
                equals,
                requires,
            } => {
                let path = path.path.clone();
                let requires = public_paths(requires);
                for (matched_path, _actual) in collect_matching_values(value, &path)
                    .into_iter()
                    .filter(|(_, matched)| values_equal(matched, &equals.0))
                {
                    let bound_requires = bind_required_paths(&path, &matched_path, &requires)
                        .unwrap_or_else(|| requires.clone());
                    let missing = missing_paths(value, &bound_requires);
                    if !missing.is_empty() {
                        let public_check = check.to_public();
                        errors.push(group_validation_error(
                            &public_check,
                            std::iter::once(matched_path.clone()).chain(missing.iter().cloned()),
                            secret_paths,
                            &format!(
                                "{matched_path} == {} requires {}",
                                equals,
                                missing.join(", ")
                            ),
                            Some(serde_json::json!({
                                "trigger": matched_path,
                                "equals": equals,
                                "requires": bound_requires
                            })),
                            Some(serde_json::json!({ "missing": missing })),
                        ));
                    }
                }
            }
        }
    }

    errors
}

fn public_paths(paths: &[crate::metadata::MetadataPathSpec]) -> Vec<String> {
    paths.iter().map(|path| path.path.clone()).collect()
}
