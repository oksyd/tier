use crate::{
    TierMetadata,
    export::{json_pretty, json_value},
};
use schemars::JsonSchema;
use serde_json::Value;

use crate::schema::annotations::{apply_metadata_annotations, redact_secret_schema_examples};
use crate::schema::model::{JsonSchemaReport, SCHEMA_EXPORT_FORMAT_VERSION};

/// Exports the JSON Schema for a configuration type.
///
/// # Examples
///
/// ```
/// use schemars::JsonSchema;
/// use serde::{Deserialize, Serialize};
/// use tier::json_schema_for;
///
/// #[derive(Debug, Serialize, Deserialize, JsonSchema)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// let schema = json_schema_for::<AppConfig>();
/// assert_eq!(schema["type"], "object");
/// ```
#[must_use]
pub fn json_schema_for<T>() -> Value
where
    T: JsonSchema,
{
    json_value(&schemars::schema_for!(T), Value::Object(Default::default()))
}

/// Exports the JSON Schema for a configuration type as pretty JSON.
#[must_use]
pub fn json_schema_pretty<T>() -> String
where
    T: JsonSchema,
{
    json_pretty(
        &json_schema_for::<T>(),
        "{\"error\":\"failed to render schema\"}",
    )
}

/// Exports the JSON Schema in a versioned machine-readable wrapper.
#[must_use]
pub fn json_schema_report<T>() -> JsonSchemaReport
where
    T: JsonSchema,
{
    JsonSchemaReport {
        format_version: SCHEMA_EXPORT_FORMAT_VERSION,
        schema: json_schema_for::<T>(),
    }
}

/// Renders the versioned JSON Schema export as JSON.
#[must_use]
pub fn json_schema_report_json<T>() -> Value
where
    T: JsonSchema,
{
    json_value(
        &json_schema_report::<T>(),
        Value::Object(Default::default()),
    )
}

/// Renders the versioned JSON Schema export as pretty JSON.
#[must_use]
pub fn json_schema_report_json_pretty<T>() -> String
where
    T: JsonSchema,
{
    json_pretty(
        &json_schema_report_json::<T>(),
        "{\"error\":\"failed to render schema report\"}",
    )
}

/// Exports the JSON Schema for a configuration type annotated with `tier` metadata.
///
/// # Examples
///
/// ```
/// use schemars::JsonSchema;
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigMetadata, FieldMetadata, TierMetadata, annotated_json_schema_for};
///
/// #[derive(Debug, Serialize, Deserialize, JsonSchema)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// impl TierMetadata for AppConfig {
///     fn metadata() -> ConfigMetadata {
///         ConfigMetadata::from_fields([
///             FieldMetadata::new("port")
///                 .env("APP_PORT")
///                 .doc("Port used for incoming traffic"),
///         ])
///     }
/// }
///
/// let schema = annotated_json_schema_for::<AppConfig>();
/// assert_eq!(schema["properties"]["port"]["x-tier-env"], "APP_PORT");
/// ```
#[must_use]
pub fn annotated_json_schema_for<T>() -> Value
where
    T: JsonSchema + TierMetadata,
{
    let mut schema = json_schema_for::<T>();
    let metadata = T::metadata();
    apply_metadata_annotations(&mut schema, &metadata);
    let snapshot = schema.clone();
    redact_secret_schema_examples(&mut schema, &snapshot);
    schema
}

/// Exports the annotated JSON Schema for a configuration type as pretty JSON.
#[must_use]
pub fn annotated_json_schema_pretty<T>() -> String
where
    T: JsonSchema + TierMetadata,
{
    json_pretty(
        &annotated_json_schema_for::<T>(),
        "{\"error\":\"failed to render schema\"}",
    )
}

/// Exports the annotated JSON Schema in a versioned machine-readable wrapper.
#[must_use]
pub fn annotated_json_schema_report<T>() -> JsonSchemaReport
where
    T: JsonSchema + TierMetadata,
{
    JsonSchemaReport {
        format_version: SCHEMA_EXPORT_FORMAT_VERSION,
        schema: annotated_json_schema_for::<T>(),
    }
}

/// Renders the versioned annotated JSON Schema export as JSON.
#[must_use]
pub fn annotated_json_schema_report_json<T>() -> Value
where
    T: JsonSchema + TierMetadata,
{
    json_value(
        &annotated_json_schema_report::<T>(),
        Value::Object(Default::default()),
    )
}

/// Renders the versioned annotated JSON Schema export as pretty JSON.
#[must_use]
pub fn annotated_json_schema_report_json_pretty<T>() -> String
where
    T: JsonSchema + TierMetadata,
{
    json_pretty(
        &annotated_json_schema_report_json::<T>(),
        "{\"error\":\"failed to render schema report\"}",
    )
}
