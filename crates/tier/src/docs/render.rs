use serde_json::Value;

use crate::{
    JsonSchema, TierMetadata,
    export::{json_pretty, json_value},
};

use super::{ENV_DOCS_FORMAT_VERSION, EnvDocOptions, EnvDocsReport, env_docs_for};

/// Renders schema-derived environment variable documentation as Markdown.
#[must_use]
pub fn env_docs_markdown<T>(options: &EnvDocOptions) -> String
where
    T: JsonSchema + TierMetadata,
{
    let docs = env_docs_for::<T>(options);
    let mut output = String::from(
        "| Path | Env | Type | Required | Default | Merge | Secret | Validation | Aliases | Deprecated | Example | Description |\n",
    );
    output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");

    for entry in docs {
        let description = markdown_cell(entry.description);
        let example = markdown_cell(entry.example);
        let deprecated = markdown_cell(entry.deprecated);
        let required = bool_cell(entry.required);
        let defaulted = bool_cell(entry.has_default);
        let secret = bool_cell(entry.secret);
        let validations = entry
            .validations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let validations = escape_markdown_cell(&validations);
        let aliases = escape_markdown_cell(&entry.aliases.join(", "));
        output.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} | `{}` | {} | {} | {} | {} | {} | {} |\n",
            entry.path,
            entry.env,
            entry.ty,
            required,
            defaulted,
            entry.merge,
            secret,
            validations,
            aliases,
            deprecated,
            example,
            description
        ));
    }

    output
}

/// Renders schema-derived environment variable documentation as machine-readable JSON.
#[must_use]
pub fn env_docs_json<T>(options: &EnvDocOptions) -> Value
where
    T: JsonSchema + TierMetadata,
{
    json_value(&env_docs_for::<T>(options), Value::Array(Vec::new()))
}

/// Renders machine-readable environment variable documentation as pretty JSON.
#[must_use]
pub fn env_docs_json_pretty<T>(options: &EnvDocOptions) -> String
where
    T: JsonSchema + TierMetadata,
{
    json_pretty(&env_docs_json::<T>(options), "[]")
}

/// Renders versioned machine-readable environment variable documentation.
#[must_use]
pub fn env_docs_report<T>(options: &EnvDocOptions) -> EnvDocsReport
where
    T: JsonSchema + TierMetadata,
{
    EnvDocsReport {
        format_version: ENV_DOCS_FORMAT_VERSION,
        entries: env_docs_for::<T>(options),
    }
}

/// Renders versioned environment variable documentation as JSON.
#[must_use]
pub fn env_docs_report_json<T>(options: &EnvDocOptions) -> Value
where
    T: JsonSchema + TierMetadata,
{
    json_value(
        &env_docs_report::<T>(options),
        Value::Object(Default::default()),
    )
}

/// Renders versioned environment variable documentation as pretty JSON.
#[must_use]
pub fn env_docs_report_json_pretty<T>(options: &EnvDocOptions) -> String
where
    T: JsonSchema + TierMetadata,
{
    json_pretty(
        &env_docs_report_json::<T>(options),
        "{\"error\":\"failed to render env docs report\"}",
    )
}

fn bool_cell(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn markdown_cell(value: Option<String>) -> String {
    escape_markdown_cell(&value.unwrap_or_default())
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('\n', " ").replace('|', "\\|")
}
