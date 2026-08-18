use std::collections::{BTreeMap, BTreeSet};

use crate::ConfigError;
use crate::metadata::paths::{render_metadata_path, validate_metadata_path};
use crate::path::render_path_with_explicit_array_segments;

use super::super::{ConfigMetadata, EnvDecoder, EnvOverrideSpec};

impl ConfigMetadata {
    pub(crate) fn canonicalize_env_decoder_paths(&mut self) -> Result<(), ConfigError> {
        let alias_source_fields = self
            .fields
            .iter()
            .filter(|field| !field.is_env_decoder_only())
            .cloned()
            .collect::<Vec<_>>();
        let alias_specs =
            ConfigMetadata::from_fields(alias_source_fields).alias_override_specs()?;

        let mut seen = BTreeMap::<String, (String, BTreeSet<usize>, EnvDecoder)>::new();
        for field in &mut self.fields {
            if !field.is_env_decoder_only() {
                continue;
            }

            let original_path = field.path.clone();
            let (canonical, explicit_array_segments) =
                ConfigMetadata::canonicalize_path_with_alias_specs_and_array_segments(
                    &original_path,
                    &field.path_explicit_array_segments,
                    &alias_specs,
                    None,
                );
            let display_path = render_path_with_explicit_array_segments(
                &original_path,
                &field.path_explicit_array_segments,
            );
            let Some(decoder) = field.env_decode else {
                return Err(ConfigError::MetadataInvalid {
                    path: original_path,
                    message: "environment decoder metadata is missing a decoder".to_owned(),
                });
            };
            if let Some((first_path, first_segments, first_decoder)) = seen.get(&canonical)
                && (first_path != &display_path
                    || first_segments != &explicit_array_segments
                    || *first_decoder != decoder)
            {
                return Err(ConfigError::MetadataConflict {
                    kind: "environment decoder",
                    name: canonical,
                    first_path: first_path.clone(),
                    second_path: display_path,
                });
            }

            seen.insert(
                canonical.clone(),
                (display_path, explicit_array_segments.clone(), decoder),
            );
            field.set_path_with_array_intent(canonical, explicit_array_segments);
        }

        self.normalize();
        Ok(())
    }

    /// Returns explicit environment variable name overrides keyed by env name.
    pub fn env_overrides(&self) -> Result<BTreeMap<String, String>, ConfigError> {
        Ok(self
            .env_override_specs()?
            .into_values()
            .map(|spec| {
                let path = render_env_override_path(&spec);
                (spec.env, path)
            })
            .collect())
    }

    pub(crate) fn env_override_specs(
        &self,
    ) -> Result<BTreeMap<String, EnvOverrideSpec>, ConfigError> {
        let mut envs = BTreeMap::new();
        let mut canonical_targets = BTreeMap::<String, String>::new();
        for field in &self.fields {
            let Some(env) = &field.env else {
                continue;
            };
            if env.is_empty() {
                return Err(ConfigError::MetadataInvalid {
                    path: field.path.clone(),
                    message: "explicit environment variable names cannot be empty".to_owned(),
                });
            }
            validate_metadata_path(&field.path)?;
            if field.path.is_empty() {
                return Err(ConfigError::MetadataInvalid {
                    path: field.path.clone(),
                    message: "explicit environment variable names cannot target the root path"
                        .to_owned(),
                });
            }
            if field.path.split('.').any(|segment| segment == "*") {
                return Err(ConfigError::MetadataInvalid {
                    path: field.path.clone(),
                    message: "explicit environment variable names cannot target wildcard paths"
                        .to_owned(),
                });
            }
            let (canonical, explicit_array_segments) = self
                .canonicalize_alias_path_with_array_segments(
                    &field.path,
                    &field.path_explicit_array_segments,
                )?;
            if let Some(first_env) = canonical_targets.insert(canonical.clone(), env.clone())
                && first_env != *env
            {
                return Err(ConfigError::MetadataConflict {
                    kind: "environment override target",
                    name: canonical,
                    first_path: first_env,
                    second_path: env.clone(),
                });
            }
            let spec = EnvOverrideSpec {
                env: env.clone(),
                path: canonical,
                explicit_array_segments,
            };
            if let Some(first) = envs.insert(env.clone(), spec.clone())
                && (first.path != spec.path
                    || first.explicit_array_segments != spec.explicit_array_segments)
            {
                return Err(ConfigError::MetadataConflict {
                    kind: "environment variable",
                    name: env.clone(),
                    first_path: render_env_override_path(&first),
                    second_path: render_env_override_path(&spec),
                });
            }
        }
        Ok(envs)
    }
}

fn render_env_override_path(spec: &EnvOverrideSpec) -> String {
    render_metadata_path(&spec.path, &spec.explicit_array_segments)
}
