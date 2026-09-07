use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
mod error;
pub(in crate::loader) use error::ValueDeError;
use serde_json::Value;

use crate::error::ConfigError;
use crate::report::ConfigReport;

use super::overrides::coerce_retry_scalars;
use super::path::normalize_external_path;
use super::unknown::find_source_for_unknown_path;
use crate::path::replace_value_at_path;

mod coercing;
mod insert;

pub(super) use self::coercing::CoercingDeserializer;
pub(crate) use self::insert::insert_path_with_shape_and_explicit_arrays;

pub(super) fn deserialize_with_path<T>(
    value: &Value,
    report: &ConfigReport,
    string_coercion_paths: &BTreeSet<String>,
) -> Result<(T, Value), ConfigError>
where
    T: DeserializeOwned,
{
    let deserialize_attempt = |value: &Value| {
        let coerced_values = RefCell::new(BTreeMap::new());
        let deserializer = CoercingDeserializer::new(
            value,
            "",
            string_coercion_paths,
            None,
            None,
            Some(&coerced_values),
        );
        let result: Result<T, serde_path_to_error::Error<ValueDeError>> =
            serde_path_to_error::deserialize(deserializer);
        (result, coerced_values.into_inner())
    };
    let finish = |config, value: &Value, coerced_values: BTreeMap<String, Value>| {
        let mut effective = value.clone();
        for (path, coerced) in coerced_values {
            let _ = replace_value_at_path(&mut effective, &path, coerced);
        }
        (config, effective)
    };

    let (result, observed_values) = deserialize_attempt(value);
    match result {
        Ok(config) => Ok(finish(config, value, observed_values)),
        Err(error) => {
            let mut retry_value = coerce_retry_scalars(
                value,
                "",
                string_coercion_paths,
                &observed_values,
                error.inner(),
            );
            for _ in 0..=string_coercion_paths.len() {
                if retry_value == *value {
                    break;
                }
                let (retry_result, coerced_values) = deserialize_attempt(&retry_value);
                match retry_result {
                    Ok(config) => return Ok(finish(config, &retry_value, coerced_values)),
                    Err(retry_error) => {
                        let next = coerce_retry_scalars(
                            &retry_value,
                            "",
                            string_coercion_paths,
                            &coerced_values,
                            retry_error.inner(),
                        );
                        if next == retry_value {
                            break;
                        }
                        retry_value = next;
                    }
                }
            }
            Err(deserialization_error(report, error))
        }
    }
}

fn deserialization_error(
    report: &ConfigReport,
    error: serde_path_to_error::Error<ValueDeError>,
) -> ConfigError {
    let path = error.path().to_string();
    let lookup_path = normalize_external_path(&path);
    let source = find_source_for_unknown_path(report, &lookup_path);
    ConfigError::Deserialize {
        path,
        provenance: source,
        message: report.redact_diagnostic_message(&lookup_path, error.inner().to_string()),
    }
}
