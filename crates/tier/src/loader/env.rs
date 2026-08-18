use std::collections::BTreeMap;
use std::ffi::OsString;

use crate::EnvDecoder;

mod binding;
mod decoder;
mod layer;
mod name;
mod state;
mod target;

use self::binding::{EnvBinding, EnvBindingConflict};

#[derive(Debug, Clone)]
pub(super) enum EnvInput {
    Process,
    Pairs(Vec<(OsString, OsString)>),
}

#[derive(Debug, Clone)]
/// Environment variable source definition.
///
/// Use `EnvSource` when environment variables should participate in the same
/// layered pipeline as defaults and files.
///
/// # Examples
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigLoader, EnvSource};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     server: ServerConfig,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct ServerConfig {
///     port: u16,
/// }
///
/// impl Default for AppConfig {
///     fn default() -> Self {
///         Self {
///             server: ServerConfig { port: 3000 },
///         }
///     }
/// }
///
/// let loaded = ConfigLoader::new(AppConfig::default())
///     .env(EnvSource::from_pairs([("APP__SERVER__PORT", "7000")]).prefix("APP"))
///     .load()?;
///
/// assert_eq!(loaded.server.port, 7000);
/// # Ok::<(), tier::ConfigError>(())
/// ```
pub struct EnvSource {
    input: EnvInput,
    prefix: Option<String>,
    separator: String,
    lowercase_segments: bool,
    bindings: BTreeMap<String, EnvBinding>,
    binding_conflicts: Vec<EnvBindingConflict>,
}

impl EnvSource {
    /// Creates a source that reads the current process environment during loading.
    ///
    /// Relevant non-Unicode variables are returned as
    /// [`ConfigError`](crate::ConfigError) instead of panicking.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_input(EnvInput::Process)
    }

    /// Creates a prefixed source that reads the current process environment during loading.
    #[must_use]
    pub fn prefixed(prefix: impl Into<String>) -> Self {
        Self::from_env().prefix(prefix)
    }

    /// Creates an environment source from explicit key/value pairs.
    ///
    /// Duplicate variable names are rejected during loading so tests and custom
    /// env adapters do not silently depend on insertion order.
    #[must_use]
    pub fn from_pairs<I, K, V>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self::from_input(EnvInput::Pairs(
            iter.into_iter()
                .map(|(key, value)| (OsString::from(key.into()), OsString::from(value.into())))
                .collect(),
        ))
    }

    /// Creates an environment source from potentially non-Unicode pairs.
    ///
    /// Encoding errors are reported when the loader runs, allowing callers that
    /// adapt platform-native environments to stay panic-free.
    #[must_use]
    pub fn from_os_pairs<I, K, V>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        Self::from_input(EnvInput::Pairs(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        ))
    }

    fn from_input(input: EnvInput) -> Self {
        Self {
            input,
            prefix: None,
            separator: "__".to_owned(),
            lowercase_segments: true,
            bindings: BTreeMap::new(),
            binding_conflicts: Vec::new(),
        }
    }

    /// Sets an environment variable prefix filter.
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets the segment separator used to map variables to paths.
    #[must_use]
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        let separator = separator.into();
        if !separator.is_empty() {
            self.separator = separator;
        }
        self
    }

    /// Preserves segment case instead of lowercasing them.
    #[must_use]
    pub fn preserve_case(mut self) -> Self {
        self.lowercase_segments = false;
        self
    }

    /// Maps an explicit environment variable name to a configuration path.
    ///
    /// This is useful for compatibility with standard operational variables
    /// such as `HTTP_PROXY` alongside application-scoped names.
    #[must_use]
    pub fn with_alias(mut self, name: impl Into<String>, path: impl Into<String>) -> Self {
        self.insert_binding(
            name.into(),
            EnvBinding {
                path: path.into(),
                decoder: None,
                fallback: false,
            },
        );
        self
    }

    /// Maps an explicit environment variable name to a configuration path and
    /// decodes it with a built-in env decoder.
    #[must_use]
    pub fn with_alias_decoder(
        mut self,
        name: impl Into<String>,
        path: impl Into<String>,
        decoder: EnvDecoder,
    ) -> Self {
        self.insert_binding(
            name.into(),
            EnvBinding {
                path: path.into(),
                decoder: Some(decoder),
                fallback: false,
            },
        );
        self
    }

    /// Registers a lower-priority compatibility env mapping for a path.
    ///
    /// Fallback env names only apply when the same configuration path was not
    /// already written by a more specific env binding from this source.
    /// Multiple fallback names that are set for the same path are rejected
    /// instead of using an implicit environment-variable ordering.
    #[must_use]
    pub fn with_fallback(mut self, name: impl Into<String>, path: impl Into<String>) -> Self {
        self.insert_binding(
            name.into(),
            EnvBinding {
                path: path.into(),
                decoder: None,
                fallback: true,
            },
        );
        self
    }

    /// Registers a lower-priority compatibility env mapping with a built-in
    /// decoder for structured values such as `NO_PROXY`.
    ///
    /// Multiple fallback names that are set for the same path are rejected
    /// instead of using an implicit environment-variable ordering.
    #[must_use]
    pub fn with_fallback_decoder(
        mut self,
        name: impl Into<String>,
        path: impl Into<String>,
        decoder: EnvDecoder,
    ) -> Self {
        self.insert_binding(
            name.into(),
            EnvBinding {
                path: path.into(),
                decoder: Some(decoder),
                fallback: true,
            },
        );
        self
    }

    fn insert_binding(&mut self, name: String, binding: EnvBinding) {
        if let Some(existing) = self.bindings.get(&name) {
            if existing != &binding {
                self.binding_conflicts.push(EnvBindingConflict {
                    name: name.clone(),
                    first: existing.clone(),
                    second: binding,
                });
            }
            return;
        }

        self.bindings.insert(name, binding);
    }
}
