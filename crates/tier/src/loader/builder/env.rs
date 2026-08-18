use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::EnvDecoder;

use super::super::ConfigLoader;

impl<T> ConfigLoader<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Registers a built-in environment decoder for a configuration path.
    ///
    /// This is useful for operational formats such as comma-separated lists or
    /// `key=value` maps that are common in environment variables but awkward to
    /// express as JSON.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> Result<(), tier::ConfigError> {
    /// use serde::{Deserialize, Serialize};
    /// use tier::{ConfigLoader, EnvDecoder, EnvSource};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    /// struct AppConfig {
    ///     no_proxy: Vec<String>,
    /// }
    ///
    /// let loaded = ConfigLoader::new(AppConfig { no_proxy: Vec::new() })
    ///     .env_decoder("no_proxy", EnvDecoder::Csv)
    ///     .env(EnvSource::from_pairs([(
    ///         "APP__NO_PROXY",
    ///         "localhost,127.0.0.1,.internal.example.com",
    ///     )]).prefix("APP"))
    ///     .load()?;
    ///
    /// assert_eq!(
    ///     loaded.no_proxy,
    ///     vec![
    ///         "localhost".to_owned(),
    ///         "127.0.0.1".to_owned(),
    ///         ".internal.example.com".to_owned(),
    ///     ]
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn env_decoder(mut self, path: impl Into<String>, decoder: EnvDecoder) -> Self {
        let path = path.into();
        self.env_decoders.insert(path, decoder);
        self
    }

    /// Registers a custom environment decoder for a configuration path.
    ///
    /// This keeps application-specific env parsing inside `tier` without
    /// requiring pre-normalization before building an [`EnvSource`](crate::EnvSource).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> Result<(), tier::ConfigError> {
    /// use serde::{Deserialize, Serialize};
    /// use serde_json::Value;
    /// use tier::{ConfigLoader, EnvSource};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    /// struct AppConfig {
    ///     no_proxy: Vec<String>,
    /// }
    ///
    /// let loaded = ConfigLoader::new(AppConfig { no_proxy: Vec::new() })
    ///     .env_decoder_with("no_proxy", |raw| {
    ///         Ok(Value::Array(
    ///             raw.split(';')
    ///                 .map(str::trim)
    ///                 .filter(|segment| !segment.is_empty())
    ///                 .map(|segment| Value::String(segment.to_owned()))
    ///                 .collect(),
    ///         ))
    ///     })
    ///     .env(EnvSource::from_pairs([("APP__NO_PROXY", "localhost;.internal")]).prefix("APP"))
    ///     .load()?;
    ///
    /// assert_eq!(loaded.no_proxy, vec!["localhost", ".internal"]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn env_decoder_with<F>(mut self, path: impl Into<String>, decoder: F) -> Self
    where
        F: Fn(&str) -> Result<Value, String> + Send + Sync + 'static,
    {
        let path = path.into();
        self.custom_env_decoders.insert(path, Arc::new(decoder));
        self
    }
}
