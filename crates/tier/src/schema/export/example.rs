use std::collections::BTreeSet;

use crate::{
    TierMetadata,
    export::{json_pretty, json_value},
};
use schemars::JsonSchema;
use serde_json::Value;

use super::json::annotated_json_schema_for;
use crate::schema::example::build_example_value;
use crate::schema::model::{ConfigExampleReport, SCHEMA_EXPORT_FORMAT_VERSION};

/// Generates a machine-readable example configuration value from schema and metadata.
///
/// # Examples
///
/// ```
/// use schemars::JsonSchema;
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigMetadata, FieldMetadata, TierMetadata, config_example_for};
///
/// #[derive(Debug, Serialize, Deserialize, JsonSchema)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// impl TierMetadata for AppConfig {
///     fn metadata() -> ConfigMetadata {
///         ConfigMetadata::from_fields([FieldMetadata::new("port").example("8080")])
///     }
/// }
///
/// let example = config_example_for::<AppConfig>();
/// assert_eq!(example["port"], 8080);
/// ```
#[must_use]
pub fn config_example_for<T>() -> Value
where
    T: JsonSchema + TierMetadata,
{
    let schema = annotated_json_schema_for::<T>();
    build_example_value(&schema, &schema, &mut BTreeSet::new(), None)
        .unwrap_or(Value::Object(Default::default()))
}

/// Renders the generated example configuration as pretty JSON.
#[must_use]
pub fn config_example_pretty<T>() -> String
where
    T: JsonSchema + TierMetadata,
{
    json_pretty(
        &config_example_for::<T>(),
        "{\"error\":\"failed to render example config\"}",
    )
}

/// Exports the generated example configuration in a versioned wrapper.
#[must_use]
pub fn config_example_report<T>() -> ConfigExampleReport
where
    T: JsonSchema + TierMetadata,
{
    ConfigExampleReport {
        format_version: SCHEMA_EXPORT_FORMAT_VERSION,
        example: config_example_for::<T>(),
    }
}

/// Renders the versioned example export as JSON.
#[must_use]
pub fn config_example_report_json<T>() -> Value
where
    T: JsonSchema + TierMetadata,
{
    json_value(
        &config_example_report::<T>(),
        Value::Object(Default::default()),
    )
}

/// Renders the versioned example export as pretty JSON.
#[must_use]
pub fn config_example_report_json_pretty<T>() -> String
where
    T: JsonSchema + TierMetadata,
{
    json_pretty(
        &config_example_report_json::<T>(),
        "{\"error\":\"failed to render example report\"}",
    )
}
