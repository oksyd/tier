use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use serde::de::value::Error as ValueDeError;
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
        result.map(|config| {
            let mut effective = value.clone();
            for (path, coerced) in coerced_values.into_inner() {
                let _ = replace_value_at_path(&mut effective, &path, coerced);
            }
            (config, effective)
        })
    };

    match deserialize_attempt(value) {
        Ok(config) => Ok(config),
        Err(error) => {
            let retry_value = coerce_retry_scalars(value, "", string_coercion_paths);
            if retry_value != *value
                && let Ok(config) = deserialize_attempt(&retry_value)
            {
                return Ok(config);
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
        message: error.inner().to_string(),
    }
}
