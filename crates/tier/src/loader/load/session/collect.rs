use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ConfigError;

use super::LoadSession;
use crate::loader::args::layer::ArgsLayerState;
use crate::loader::env_decoder::{canonicalize_custom_env_decoders, canonicalize_env_decoders};
use crate::loader::file::load_file_layer;
use crate::loader::merge::merged_shape_from_layers;
use crate::loader::{Layer, PendingCustomLayer, SourceKind, SourceTrace};

impl<T> LoadSession<T>
where
    T: Serialize + DeserializeOwned,
{
    pub(super) fn collect_layers(&mut self) -> Result<(), ConfigError> {
        self.add_default_layer()?;
        self.add_file_layers()?;
        self.add_custom_layers()?;
        self.prepare_env_metadata()?;
        self.add_env_layers()?;
        self.add_arg_layers()?;
        self.add_typed_arg_layers()
    }

    fn add_default_layer(&mut self) -> Result<(), ConfigError> {
        let layer = Layer::from_serializable(
            SourceTrace::new(SourceKind::Default, "defaults"),
            &self.defaults,
        )?;
        self.push_canonical_layer(layer)
    }

    fn add_file_layers(&mut self) -> Result<(), ConfigError> {
        let mut files = std::mem::take(&mut self.files);
        if let Some(parsed) = &self.parsed_args {
            files.extend(parsed.files.clone());
        }

        for file in files {
            if let Some(layer) = load_file_layer(file, self.profile.as_deref())? {
                self.push_canonical_layer(layer)?;
            }
        }

        Ok(())
    }

    fn add_custom_layers(&mut self) -> Result<(), ConfigError> {
        for pending in std::mem::take(&mut self.custom_layers) {
            self.recanonicalize_metadata()?;
            let layer = match pending {
                PendingCustomLayer::Immediate(layer) => layer,
                PendingCustomLayer::DeferredPatch(patch) => patch.into_layer_with_shape(
                    merged_shape_from_layers(&self.defaults_shape, &self.layers, &self.metadata)?,
                )?,
            };
            self.push_canonical_layer(layer)?;
        }

        Ok(())
    }

    fn prepare_env_metadata(&mut self) -> Result<(), ConfigError> {
        self.recanonicalize_metadata()?;
        let _ = self.metadata.env_overrides()?;
        Ok(())
    }

    fn add_env_layers(&mut self) -> Result<(), ConfigError> {
        for env_source in std::mem::take(&mut self.env_sources) {
            self.recanonicalize_metadata()?;
            let env_decoders =
                canonicalize_env_decoders(&self.env_decoders, &self.metadata, &self.layers)?;
            let custom_env_decoders = canonicalize_custom_env_decoders(
                &self.custom_env_decoders,
                &self.metadata,
                &self.layers,
            )?;
            let runtime_shape =
                merged_shape_from_layers(&self.defaults_shape, &self.layers, &self.metadata)?;
            if let Some(layer) = env_source.into_layer(
                &self.metadata,
                &env_decoders,
                &custom_env_decoders,
                &self.layers,
                &runtime_shape,
            )? {
                self.push_canonical_layer(layer)?;
            }
        }

        Ok(())
    }

    fn add_arg_layers(&mut self) -> Result<(), ConfigError> {
        let Some(parsed) = self.parsed_args.take() else {
            return Ok(());
        };
        if parsed.overrides.is_empty() {
            return Ok(());
        }

        self.recanonicalize_metadata()?;
        let runtime_shape =
            merged_shape_from_layers(&self.defaults_shape, &self.layers, &self.metadata)?;
        let mut state = ArgsLayerState::new();
        for override_ in parsed.overrides {
            let (path, explicit_array_segments) = self
                .metadata
                .canonicalize_alias_path_with_array_segments_for_shape(
                    &override_.path,
                    &override_.explicit_array_segments,
                    Some(&runtime_shape),
                )?;
            state.insert_override(
                &override_.source_name,
                &path,
                override_.parsed,
                override_.error_arg,
                Some(&runtime_shape),
                &explicit_array_segments,
            )?;
        }

        if let Some(layer) = state.into_layer() {
            self.push_canonical_layer(layer)?;
        }

        Ok(())
    }

    fn add_typed_arg_layers(&mut self) -> Result<(), ConfigError> {
        for patch in std::mem::take(&mut self.typed_arg_layers) {
            self.recanonicalize_metadata()?;
            let layer = patch.into_layer_with_shape(merged_shape_from_layers(
                &self.defaults_shape,
                &self.layers,
                &self.metadata,
            )?)?;
            self.push_canonical_layer(layer)?;
        }

        Ok(())
    }
}
