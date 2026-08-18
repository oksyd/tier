use serde::Serialize;
use serde::de::DeserializeOwned;

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

impl<T> ConfigLoader<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Creates a loader with the provided in-code defaults.
    #[must_use]
    pub fn new(defaults: T) -> Self {
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
        }
    }
}
