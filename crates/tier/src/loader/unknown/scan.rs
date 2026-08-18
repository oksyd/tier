use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use serde::de::value::Error as ValueDeError;
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
        message: error.to_string(),
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
}

fn scan_unknown_field_paths<T>(
    value: &Value,
    string_coercion_paths: &BTreeSet<String>,
) -> UnknownFieldScan<T>
where
    T: DeserializeOwned,
{
    let ignored = RefCell::new(Vec::new());
    let known_paths = RefCell::new(BTreeSet::new());
    let deserializer = crate::loader::de::CoercingDeserializer::new(
        value,
        "",
        string_coercion_paths,
        Some(&known_paths),
        Some(&ignored),
        None,
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

    let retry_value = coerce_retry_scalars(value, "", string_coercion_paths);
    if retry_value == *value {
        return scan;
    }

    let retry_scan = scan_unknown_field_paths::<T>(&retry_value, string_coercion_paths);
    if retry_scan.result.is_ok() {
        retry_scan
    } else {
        scan
    }
}
