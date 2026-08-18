use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::ConfigError;
use crate::metadata::ConfigMetadata;
use crate::report::{ConfigReport, ConfigWarning};

use crate::loader::UnknownFieldPolicy;
use crate::loader::unknown::{
    collect_unknown_fields, collect_unknown_fields_best_effort,
    collect_unknown_fields_from_metadata_scope, deserialize_error_scope, error_path_for_scope,
};

pub(super) fn pre_deserialize_unknown_fields_error<T>(
    policy: UnknownFieldPolicy,
    merged: &Value,
    metadata: &ConfigMetadata,
    suggestion_paths: &BTreeMap<String, String>,
    report: &ConfigReport,
    string_coercion_paths: &BTreeSet<String>,
    error: &ConfigError,
) -> Option<ConfigError>
where
    T: DeserializeOwned,
{
    if matches!(policy, UnknownFieldPolicy::Allow) {
        return None;
    }

    let mut unknown_fields = collect_unknown_fields_best_effort::<T>(
        merged,
        suggestion_paths,
        report,
        string_coercion_paths,
    );
    if unknown_fields.is_empty() && !metadata.fields().is_empty() {
        unknown_fields = collect_unknown_fields_from_metadata_scope(
            merged,
            metadata,
            suggestion_paths,
            report,
            deserialize_error_scope(error_path_for_scope(error)),
        );
    }

    (!unknown_fields.is_empty()).then_some(ConfigError::UnknownFields {
        fields: unknown_fields,
    })
}

pub(super) fn apply_unknown_field_policy<T>(
    policy: UnknownFieldPolicy,
    merged: &Value,
    suggestion_paths: &BTreeMap<String, String>,
    report: &mut ConfigReport,
    string_coercion_paths: &BTreeSet<String>,
) -> Result<(), ConfigError>
where
    T: DeserializeOwned,
{
    if matches!(policy, UnknownFieldPolicy::Allow) {
        return Ok(());
    }

    let unknown_fields =
        collect_unknown_fields::<T>(merged, suggestion_paths, report, string_coercion_paths)?;
    if unknown_fields.is_empty() {
        return Ok(());
    }

    match policy {
        UnknownFieldPolicy::Allow => Ok(()),
        UnknownFieldPolicy::Warn => {
            for field in unknown_fields {
                report.record_warning(ConfigWarning::UnknownField(field));
            }
            Ok(())
        }
        UnknownFieldPolicy::Deny => Err(ConfigError::UnknownFields {
            fields: unknown_fields,
        }),
    }
}
