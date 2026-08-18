mod annotations;
mod core;
pub(crate) mod count;
mod example;
mod export;
mod model;
mod path;
#[cfg(feature = "toml")]
mod toml;

pub(crate) use self::core::{
    dynamic_object_placeholder, inlined_schema_ref, merged_object_level_property_names,
    resolve_schema_ref, schema_type_label,
};
pub(crate) use self::example::{
    allows_additional_array_items_for_schema, dynamic_object_placeholder_for_schema,
    legacy_additional_items_for_schema, required_contains_additional_items_for_docs,
};
#[cfg(feature = "toml")]
pub use self::export::config_example_toml;
pub use self::export::{
    annotated_json_schema_for, annotated_json_schema_pretty, annotated_json_schema_report,
    annotated_json_schema_report_json, annotated_json_schema_report_json_pretty,
    config_example_for, config_example_pretty, config_example_report, config_example_report_json,
    config_example_report_json_pretty, json_schema_for, json_schema_pretty, json_schema_report,
    json_schema_report_json, json_schema_report_json_pretty,
};
pub use self::model::{ConfigExampleReport, JsonSchemaReport, SCHEMA_EXPORT_FORMAT_VERSION};
pub(crate) use self::path::schema_path_explicit_array_segments;

/// Re-export of `schemars::JsonSchema` used by `tier` schema helpers.
pub use schemars::JsonSchema;
