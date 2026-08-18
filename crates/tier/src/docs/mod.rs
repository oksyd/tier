use crate::{JsonSchema, TierMetadata, json_schema_for};

mod collect;
mod metadata;
mod model;
mod render;

use self::collect::collect_env_docs;
use self::metadata::{apply_field_metadata, merge_duplicate_env_docs};

pub use self::model::{ENV_DOCS_FORMAT_VERSION, EnvDocEntry, EnvDocOptions, EnvDocsReport};
pub use self::render::{
    env_docs_json, env_docs_json_pretty, env_docs_markdown, env_docs_report, env_docs_report_json,
    env_docs_report_json_pretty,
};

/// Generates environment variable documentation rows from a configuration schema.
///
/// # Examples
///
/// ```
/// use schemars::JsonSchema;
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigMetadata, EnvDocOptions, FieldMetadata, TierMetadata, env_docs_for};
///
/// #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// struct AppConfig {
///     server: ServerConfig,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// struct ServerConfig {
///     port: u16,
/// }
///
/// impl TierMetadata for AppConfig {
///     fn metadata() -> ConfigMetadata {
///         ConfigMetadata::from_fields([
///             FieldMetadata::new("server.port")
///                 .env("APP_SERVER_PORT")
///                 .doc("Port used for incoming traffic"),
///         ])
///     }
/// }
///
/// let docs = env_docs_for::<AppConfig>(&EnvDocOptions::prefixed("APP"));
/// assert_eq!(docs[0].path, "server.port");
/// assert_eq!(docs[0].env, "APP_SERVER_PORT");
/// ```
#[must_use]
pub fn env_docs_for<T>(options: &EnvDocOptions) -> Vec<EnvDocEntry>
where
    T: JsonSchema + TierMetadata,
{
    let schema = json_schema_for::<T>();
    let mut docs = Vec::new();
    collect_env_docs(&schema, &mut docs);
    let metadata = T::metadata();
    docs.sort_by(|left, right| left.path.cmp(&right.path));
    docs = merge_duplicate_env_docs(docs);

    for entry in &mut docs {
        apply_field_metadata(entry, &schema, &metadata, options);
    }

    docs
}
