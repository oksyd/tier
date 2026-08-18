use std::collections::BTreeSet;

use crate::loader::path::try_normalize_external_path_with_explicit_arrays;
use crate::loader::secret_path::SecretPathSpec;
use crate::{ConfigError, ConfigMetadata};

pub(super) fn normalize_secret_registration_paths(
    secret_paths: &BTreeSet<SecretPathSpec>,
    metadata: &ConfigMetadata,
) -> Result<BTreeSet<SecretPathSpec>, ConfigError> {
    let mut normalized = secret_paths
        .iter()
        .map(normalize_secret_registration_path)
        .collect::<Result<BTreeSet<_>, _>>()?;
    normalized.extend(metadata_secret_path_specs(metadata));
    Ok(normalized)
}

fn normalize_secret_registration_path(
    spec: &SecretPathSpec,
) -> Result<SecretPathSpec, ConfigError> {
    let (normalized, explicit_array_segments) =
        try_normalize_external_path_with_explicit_arrays(spec.path()).map_err(|message| {
            ConfigError::MetadataInvalid {
                path: spec.path().to_owned(),
                message: format!("invalid secret path: {message}"),
            }
        })?;
    if normalized.is_empty() {
        return Err(ConfigError::MetadataInvalid {
            path: spec.path().to_owned(),
            message: "invalid secret path: configuration path cannot be empty".to_owned(),
        });
    }
    Ok(SecretPathSpec::from_normalized(
        normalized,
        explicit_array_segments,
    ))
}

fn metadata_secret_path_specs(
    metadata: &ConfigMetadata,
) -> impl Iterator<Item = SecretPathSpec> + '_ {
    metadata
        .fields()
        .iter()
        .filter(|field| field.secret)
        .map(|field| {
            SecretPathSpec::from_normalized(
                field.path.clone(),
                field.path_explicit_array_segments.clone(),
            )
        })
}
