use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::ConfigError;
use crate::metadata::{ConfigMetadata, EnvDecoder};
use crate::patch::DeferredPatchLayer;

mod collect;
mod finish;

use crate::loader::args::{ParsedArgs, parse_args};
use crate::loader::canonical::{canonicalize_layer_paths, canonicalize_metadata_against_layers};
use crate::loader::env::EnvSource;
use crate::loader::file::FileSource;
use crate::loader::merge::{ensure_root_object, merged_shape_from_layers};
use crate::loader::migration_runtime::{
    normalize_version_registration_path, validate_config_migrations,
};
use crate::loader::secret_path::SecretPathSpec;
use crate::loader::{
    ConfigLoader, ConfigMigration, CustomEnvDecoder, Layer, LoadedConfig, NamedNormalizer,
    NamedValidator, PendingCustomLayer, UnknownFieldPolicy,
};

pub(super) struct LoadSession<T> {
    defaults: T,
    files: Vec<FileSource>,
    env_sources: Vec<EnvSource>,
    custom_layers: Vec<PendingCustomLayer>,
    typed_arg_layers: Vec<DeferredPatchLayer>,
    metadata: ConfigMetadata,
    secret_paths: BTreeSet<SecretPathSpec>,
    normalizers: Vec<NamedNormalizer<T>>,
    validators: Vec<NamedValidator<T>>,
    unknown_field_policy: UnknownFieldPolicy,
    env_decoders: BTreeMap<String, EnvDecoder>,
    custom_env_decoders: BTreeMap<String, CustomEnvDecoder>,
    config_version: Option<(String, u32)>,
    migrations: Vec<ConfigMigration>,
    parsed_args: Option<ParsedArgs>,
    profile: Option<String>,
    defaults_shape: Value,
    layers: Vec<Layer>,
}

impl<T> LoadSession<T>
where
    T: Serialize + DeserializeOwned,
{
    pub(super) fn prepare(loader: ConfigLoader<T>) -> Result<Self, ConfigError> {
        let ConfigLoader {
            defaults,
            files,
            env_sources,
            args_source,
            custom_layers,
            typed_arg_layers,
            mut metadata,
            secret_paths,
            normalizers,
            validators,
            profile,
            unknown_field_policy,
            env_decoders,
            custom_env_decoders,
            config_version,
            migrations,
        } = loader;

        metadata.canonicalize_env_decoder_paths()?;
        metadata.validate_paths()?;
        if let Some((path, _)) = &config_version {
            let _ = normalize_version_registration_path(path)?;
        }
        validate_config_migrations(&migrations)?;
        if !migrations.is_empty() && config_version.is_none() {
            return Err(ConfigError::MetadataInvalid {
                path: String::new(),
                message:
                    "configuration migrations require ConfigLoader::config_version(...) to be set"
                        .to_owned(),
            });
        }

        let parsed_args = args_source.map(parse_args).transpose()?;
        let profile = parsed_args
            .as_ref()
            .and_then(|args| args.profile.clone())
            .or(profile);
        let defaults_shape = serde_json::to_value(&defaults)?;
        ensure_root_object(&defaults_shape)?;

        Ok(Self {
            defaults,
            files,
            env_sources,
            custom_layers,
            typed_arg_layers,
            metadata,
            secret_paths,
            normalizers,
            validators,
            unknown_field_policy,
            env_decoders,
            custom_env_decoders,
            config_version,
            migrations,
            parsed_args,
            profile,
            defaults_shape,
            layers: Vec::new(),
        })
    }

    pub(super) fn load(mut self) -> Result<LoadedConfig<T>, ConfigError> {
        self.collect_layers()?;
        self.finish()
    }

    fn push_canonical_layer(&mut self, layer: Layer) -> Result<(), ConfigError> {
        let shape = merged_shape_from_layers(&self.defaults_shape, &self.layers, &self.metadata)?;
        self.layers
            .push(canonicalize_layer_paths(layer, &self.metadata, &shape)?);
        Ok(())
    }

    fn recanonicalize_metadata(&mut self) -> Result<(), ConfigError> {
        self.metadata = canonicalize_metadata_against_layers(&self.metadata, &self.layers)?;
        Ok(())
    }
}
