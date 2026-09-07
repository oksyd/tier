use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::loader::de::ValueDeError;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{ConfigError, UnknownField};
use crate::report::ConfigReport;

use super::suggest::{merge_suggestion_paths, unknown_fields_from_paths};
use crate::loader::overrides::coerce_retry_scalars;
use crate::loader::path::normalize_external_path;

pub(in crate::loader) fn collect_unknown_fields<T>(
    value: &Value,
    suggestion_paths: &BTreeMap<String, String>,
    report: &ConfigReport,
    string_coercion_paths: &BTreeSet<String>,
) -> Result<Vec<UnknownField>, ConfigError>
where
    T: DeserializeOwned,
{
    let scan = scan_unknown_field_paths_with_retry::<T>(value, string_coercion_paths);
    scan.result.map_err(|error| ConfigError::Deserialize {
        path: "<unknown>".to_owned(),
        provenance: None,
        message: report.redact_diagnostic_message("", error.to_string()),
    })?;

    Ok(unknown_fields_from_paths(
        scan.ignored,
        &merge_suggestion_paths(suggestion_paths, &scan.known_paths),
        report,
    ))
}

pub(in crate::loader) fn collect_unknown_fields_best_effort<T>(
    value: &Value,
    suggestion_paths: &BTreeMap<String, String>,
    report: &ConfigReport,
    string_coercion_paths: &BTreeSet<String>,
) -> Vec<UnknownField>
where
    T: DeserializeOwned,
{
    let scan = scan_unknown_field_paths_with_retry::<T>(value, string_coercion_paths);
    unknown_fields_from_paths(
        scan.ignored,
        &merge_suggestion_paths(suggestion_paths, &scan.known_paths),
        report,
    )
}

struct UnknownFieldScan<T> {
    ignored: Vec<String>,
    known_paths: BTreeSet<String>,
    result: Result<T, ValueDeError>,
    observed_values: BTreeMap<String, Value>,
}

fn scan_unknown_field_paths<T>(
    value: &Value,
    string_coercion_paths: &BTreeSet<String>,
) -> UnknownFieldScan<T>
where
    T: DeserializeOwned,
{
    let ignored = RefCell::new(Vec::new());
    let observed_values = RefCell::new(BTreeMap::new());
    let known_paths = RefCell::new(BTreeSet::new());
    let deserializer = crate::loader::de::CoercingDeserializer::new(
        value,
        "",
        string_coercion_paths,
        Some(&known_paths),
        Some(&ignored),
        Some(&observed_values),
    );
    let result = serde_ignored::deserialize(deserializer, |path| {
        ignored
            .borrow_mut()
            .push(normalize_external_path(&path.to_string()))
    });
    let mut ignored = ignored.into_inner();
    ignored.sort();
    ignored.dedup();
    UnknownFieldScan {
        ignored,
        known_paths: known_paths.into_inner(),
        result,
        observed_values: observed_values.into_inner(),
    }
}

fn scan_unknown_field_paths_with_retry<T>(
    value: &Value,
    string_coercion_paths: &BTreeSet<String>,
) -> UnknownFieldScan<T>
where
    T: DeserializeOwned,
{
    let scan = scan_unknown_field_paths::<T>(value, string_coercion_paths);
    if scan.result.is_ok() {
        return scan;
    }

    let Some(error) = scan.result.as_ref().err() else {
        return scan;
    };
    let mut retry_value = coerce_retry_scalars(
        value,
        "",
        string_coercion_paths,
        &scan.observed_values,
        error,
    );
    for _ in 0..=string_coercion_paths.len() {
        if retry_value == *value {
            break;
        }
        let retry_scan = scan_unknown_field_paths::<T>(&retry_value, string_coercion_paths);
        let Some(error) = retry_scan.result.as_ref().err() else {
            return retry_scan;
        };
        let next = coerce_retry_scalars(
            &retry_value,
            "",
            string_coercion_paths,
            &retry_scan.observed_values,
            error,
        );
        if next == retry_value {
            break;
        }
        retry_value = next;
    }
    scan
}
