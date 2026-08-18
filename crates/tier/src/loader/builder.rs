use serde::Serialize;
use serde_json::Value;

use crate::ConfigMetadata;

use super::{ConfigLoader, UnknownFieldPolicy};

mod env;
mod hooks;
mod metadata;
mod migrations;
mod patch;
#[cfg(feature = "schema")]
mod schema;
mod sources;

impl<T> ConfigLoader<T> {
    /// Creates a loader with serializable in-code defaults.
    ///
    /// This constructor serializes `defaults` into the internal document. Use
    /// [`Self::from_value`] for deserialize-only types and configurations that
    /// contain [`crate::Secret`] fields.
    #[must_use]
    pub fn new(defaults: T) -> Self
    where
        T: Serialize,
    {
        Self::from_defaults_result(serde_json::to_value(defaults))
    }

    /// Creates a loader from an explicit JSON-like default configuration document.
    ///
    /// This constructor does not require the target type to implement [`Serialize`]
    /// and is therefore the preferred entry point for configurations containing
    /// non-serializable secret fields.
    #[must_use]
    pub fn from_value(defaults: Value) -> Self {
        Self::from_defaults_result(Ok(defaults))
    }

    fn from_defaults_result(defaults: Result<Value, serde_json::Error>) -> Self {
        Self {
            defaults,
            files: Vec::new(),
            env_sources: Vec::new(),
            args_source: None,
            custom_layers: Vec::new(),
            typed_arg_layers: Vec::new(),
            metadata: ConfigMetadata::default(),
            secret_paths: Default::default(),
            normalizers: Vec::new(),
            validators: Vec::new(),
            profile: None,
            unknown_field_policy: UnknownFieldPolicy::Deny,
            env_decoders: Default::default(),
            custom_env_decoders: Default::default(),
            config_version: None,
            migrations: Vec::new(),
            target: std::marker::PhantomData,
        }
    }
}
