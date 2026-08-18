use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::error::ConfigError;
use crate::path::render_path_with_explicit_array_segments;
use crate::{ConfigMetadata, EnvDecoder};

use super::canonical::try_canonicalize_runtime_path_across_layers_with_explicit_arrays;
use super::path::try_normalize_external_path_with_explicit_arrays;
use super::{CustomEnvDecoder, Layer};

pub(super) fn canonicalize_custom_env_decoders(
    decoders: &BTreeMap<String, CustomEnvDecoder>,
    metadata: &ConfigMetadata,
    layers: &[Layer],
) -> Result<BTreeMap<String, CustomEnvDecoder>, ConfigError> {
    let mut canonicalized = BTreeMap::new();
    let mut origins = BTreeMap::<String, (String, BTreeSet<usize>)>::new();

    for (path, decoder) in decoders {
        let (registered_path, explicit_array_segments) = normalize_decoder_registration_path(path)?;
        let normalized = try_canonicalize_runtime_path_across_layers_with_explicit_arrays(
            &registered_path,
            &explicit_array_segments,
            layers,
        )
        .map_err(|message| invalid_decoder_registration_path(path, message))?;
        let (canonical, canonical_explicit_array_segments) = metadata
            .canonicalize_alias_path_with_array_segments(&normalized, &explicit_array_segments)?;
        let display_path =
            render_path_with_explicit_array_segments(&normalized, &explicit_array_segments);
        if let Some((first_path, first_segments)) = origins.get(&canonical)
            && (first_path != &display_path || first_segments != &canonical_explicit_array_segments)
        {
            return Err(ConfigError::MetadataConflict {
                kind: "environment decoder",
                name: canonical,
                first_path: first_path.clone(),
                second_path: display_path,
            });
        }

        origins.insert(
            canonical.clone(),
            (display_path, canonical_explicit_array_segments),
        );
        canonicalized.insert(canonical, Arc::clone(decoder));
    }

    Ok(canonicalized)
}

pub(super) fn canonicalize_env_decoders(
    decoders: &BTreeMap<String, EnvDecoder>,
    metadata: &ConfigMetadata,
    layers: &[Layer],
) -> Result<BTreeMap<String, EnvDecoder>, ConfigError> {
    let mut canonicalized = BTreeMap::new();
    let mut origins = BTreeMap::<String, (String, BTreeSet<usize>, EnvDecoder)>::new();

    for (path, decoder) in decoders {
        let (registered_path, explicit_array_segments) = normalize_decoder_registration_path(path)?;
        let normalized = try_canonicalize_runtime_path_across_layers_with_explicit_arrays(
            &registered_path,
            &explicit_array_segments,
            layers,
        )
        .map_err(|message| invalid_decoder_registration_path(path, message))?;
        let (canonical, canonical_explicit_array_segments) = metadata
            .canonicalize_alias_path_with_array_segments(&normalized, &explicit_array_segments)?;
        let display_path =
            render_path_with_explicit_array_segments(&normalized, &explicit_array_segments);
        if let Some((first_path, first_segments, first_decoder)) = origins.get(&canonical)
            && (first_path != &display_path
                || first_segments != &canonical_explicit_array_segments
                || *first_decoder != *decoder)
        {
            return Err(ConfigError::MetadataConflict {
                kind: "environment decoder",
                name: canonical,
                first_path: first_path.clone(),
                second_path: display_path,
            });
        }

        origins.insert(
            canonical.clone(),
            (display_path, canonical_explicit_array_segments, *decoder),
        );
        canonicalized.insert(canonical, *decoder);
    }

    Ok(canonicalized)
}

fn normalize_decoder_registration_path(
    path: &str,
) -> Result<(String, std::collections::BTreeSet<usize>), ConfigError> {
    let normalized = try_normalize_external_path_with_explicit_arrays(path).map_err(|message| {
        ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: format!("invalid environment decoder path: {message}"),
        }
    })?;
    if normalized.0.is_empty() {
        return Err(ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: "invalid environment decoder path: configuration path cannot be empty"
                .to_owned(),
        });
    }
    Ok(normalized)
}

fn invalid_decoder_registration_path(path: &str, message: String) -> ConfigError {
    ConfigError::MetadataInvalid {
        path: path.to_owned(),
        message: format!("invalid environment decoder path: {message}"),
    }
}
