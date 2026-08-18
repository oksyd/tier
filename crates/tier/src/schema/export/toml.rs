use crate::TierMetadata;
use schemars::JsonSchema;

use super::example::config_example_for;
use crate::schema::toml::render_example_toml;

/// Renders the generated example configuration as commented TOML.
///
/// # Examples
///
/// ```
/// use schemars::JsonSchema;
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigMetadata, FieldMetadata, TierMetadata, config_example_toml};
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
///                 .doc("Port used for incoming traffic")
///                 .example("8080"),
///         ])
///     }
/// }
///
/// let example = config_example_toml::<AppConfig>();
/// assert!(example.contains("8080"));
/// assert!(example.contains("incoming traffic"));
/// ```
#[must_use]
pub fn config_example_toml<T>() -> String
where
    T: JsonSchema + TierMetadata,
{
    let example = config_example_for::<T>();
    let metadata = T::metadata();
    render_example_toml(&example, &metadata)
}
