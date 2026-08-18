use std::collections::BTreeMap;
use std::ffi::OsString;

use serde_json::Value;

use crate::path::{concrete_paths_overlap, get_value_at_path};
use crate::{ConfigError, ConfigMetadata, EnvDecoder, EnvironmentVariableComponent};

mod context;
mod insert;

use super::binding::EnvBinding;
use super::name::path_for_env_var;
use super::state::EnvLayerState;
use super::target::{
    canonicalize_runtime_env_target_path, validate_binding_names,
    validate_binding_override_conflicts, validate_binding_paths,
};
use super::{EnvInput, EnvSource};
use crate::loader::{CustomEnvDecoder, Layer};

use self::context::EnvLayerContext;
use self::insert::{EnvInsertTarget, insert_env_value};

struct EnvVarConflict {
    name: String,
}

struct EnvResolveContext<'a> {
    prefix: Option<&'a str>,
    separator: &'a str,
    lowercase_segments: bool,
    bindings: &'a BTreeMap<String, EnvBinding>,
    env_overrides: &'a BTreeMap<String, crate::metadata::EnvOverrideSpec>,
}

impl EnvSource {
    pub(in crate::loader) fn into_layer(
        self,
        metadata: &ConfigMetadata,
        env_decoders: &BTreeMap<String, EnvDecoder>,
        custom_env_decoders: &BTreeMap<String, CustomEnvDecoder>,
        runtime_layers: &[Layer],
        runtime_shape: &Value,
    ) -> Result<Option<Layer>, ConfigError> {
        let EnvSource {
            input,
            prefix,
            separator,
            lowercase_segments,
            bindings,
            binding_conflicts,
        } = self;
        validate_binding_names(&bindings)?;
        if let Some(conflict) = binding_conflicts.into_iter().next() {
            let path = conflict.second.path.clone();
            return Err(ConfigError::InvalidEnv {
                name: conflict.name,
                path,
                message: format!(
                    "conflicting explicit env bindings target `{}` and `{}`",
                    conflict.first.path, conflict.second.path
                ),
            });
        }
        validate_binding_paths(&bindings, metadata, runtime_layers, runtime_shape)?;
        let env_overrides = metadata.env_override_specs()?;
        let env_context = EnvResolveContext {
            prefix: prefix.as_deref(),
            separator: &separator,
            lowercase_segments,
            bindings: &bindings,
            env_overrides: &env_overrides,
        };
        let (vars, var_conflicts) = resolve_env_input(input, &env_context)?;
        if let Some(conflict) = var_conflicts.into_iter().next() {
            return Err(ConfigError::InvalidEnv {
                name: conflict.name,
                path: String::new(),
                message: "environment source contains duplicate variable names".to_owned(),
            });
        }
        let context = EnvLayerContext {
            metadata,
            env_decoders,
            custom_env_decoders,
            runtime_layers,
            runtime_shape,
        };
        validate_binding_override_conflicts(
            &bindings,
            &env_overrides,
            metadata,
            runtime_layers,
            runtime_shape,
        )?;
        let mut state = EnvLayerState::new();
        let mut fallback_vars = Vec::new();

        for (name, raw_value) in vars {
            if let Some(binding) = bindings.get(&name) {
                if binding.fallback {
                    fallback_vars.push((name, raw_value, binding.clone()));
                } else {
                    insert_env_value(
                        &context,
                        &mut state,
                        &name,
                        &raw_value,
                        EnvInsertTarget {
                            path: &binding.path,
                            external_path: Some(binding.path.clone()),
                            explicit_array_segments: None,
                        },
                        binding.decoder,
                    )?;
                }
                continue;
            }

            if let Some(spec) = env_overrides.get(&name) {
                insert_env_value(
                    &context,
                    &mut state,
                    &name,
                    &raw_value,
                    EnvInsertTarget {
                        path: &spec.path,
                        external_path: Some(spec.path.clone()),
                        explicit_array_segments: Some(&spec.explicit_array_segments),
                    },
                    None,
                )?;
                continue;
            }

            let Some(path) =
                path_for_env_var(&name, prefix.as_deref(), &separator, lowercase_segments)
                    .map_err(|error| ConfigError::InvalidEnv {
                        name: name.clone(),
                        path: error.path,
                        message: error.message,
                    })?
            else {
                continue;
            };
            insert_env_value(
                &context,
                &mut state,
                &name,
                &raw_value,
                EnvInsertTarget {
                    path: &path,
                    external_path: Some(path.clone()),
                    explicit_array_segments: None,
                },
                None,
            )?;
        }

        let mut claimed_fallback_paths = BTreeMap::new();
        for (name, raw_value, binding) in fallback_vars {
            let normalized = canonicalize_runtime_env_target_path(
                &name,
                &binding.path,
                metadata,
                runtime_layers,
                &state.root,
                runtime_shape,
            )?;
            if normalized.is_empty() {
                continue;
            }
            reject_conflicting_fallback(&name, &normalized, &claimed_fallback_paths)?;
            if get_value_at_path(&state.root, &normalized).is_some() {
                continue;
            }
            claimed_fallback_paths.insert(normalized.clone(), name.clone());
            insert_env_value(
                &context,
                &mut state,
                &name,
                &raw_value,
                EnvInsertTarget {
                    path: &binding.path,
                    external_path: Some(binding.path.clone()),
                    explicit_array_segments: None,
                },
                binding.decoder,
            )?;
        }

        Ok(state.into_layer())
    }
}

fn resolve_env_input(
    input: EnvInput,
    context: &EnvResolveContext<'_>,
) -> Result<(BTreeMap<String, String>, Vec<EnvVarConflict>), ConfigError> {
    match input {
        EnvInput::Process => resolve_env_pairs(std::env::vars_os(), true, context),
        EnvInput::Pairs(pairs) => resolve_env_pairs(pairs, false, context),
    }
}

fn resolve_env_pairs(
    pairs: impl IntoIterator<Item = (OsString, OsString)>,
    process_input: bool,
    context: &EnvResolveContext<'_>,
) -> Result<(BTreeMap<String, String>, Vec<EnvVarConflict>), ConfigError> {
    let filtered = context
        .prefix
        .map(|prefix| crate::env_name::normalize_env_prefix(prefix, context.separator))
        .is_some_and(|prefix| !prefix.is_empty());
    let mut vars = BTreeMap::new();
    let mut conflicts = Vec::new();

    for (raw_name, raw_value) in pairs {
        let name = match raw_name.into_string() {
            Ok(name) => name,
            Err(_name) if process_input && filtered => continue,
            Err(name) => {
                return Err(ConfigError::NonUnicodeEnvironment {
                    name,
                    component: EnvironmentVariableComponent::Name,
                });
            }
        };
        if process_input && !process_env_name_is_relevant(&name, context)? {
            continue;
        }
        let value = raw_value
            .into_string()
            .map_err(|_| ConfigError::NonUnicodeEnvironment {
                name: OsString::from(&name),
                component: EnvironmentVariableComponent::Value,
            })?;
        if vars.insert(name.clone(), value).is_some() {
            conflicts.push(EnvVarConflict { name });
        }
    }

    Ok((vars, conflicts))
}

fn process_env_name_is_relevant(
    name: &str,
    context: &EnvResolveContext<'_>,
) -> Result<bool, ConfigError> {
    if context.bindings.contains_key(name) || context.env_overrides.contains_key(name) {
        return Ok(true);
    }

    path_for_env_var(
        name,
        context.prefix,
        context.separator,
        context.lowercase_segments,
    )
    .map(|path| path.is_some())
    .map_err(|error| ConfigError::InvalidEnv {
        name: name.to_owned(),
        path: error.path,
        message: error.message,
    })
}

fn reject_conflicting_fallback(
    name: &str,
    path: &str,
    claimed_fallback_paths: &BTreeMap<String, String>,
) -> Result<(), ConfigError> {
    for (existing_path, existing_name) in claimed_fallback_paths {
        if concrete_paths_overlap(existing_path, path) {
            return Err(ConfigError::InvalidEnv {
                name: name.to_owned(),
                path: path.to_owned(),
                message: format!(
                    "conflicting fallback environment variables `{existing_name}` and `{name}` target overlapping configuration paths `{existing_path}` and `{path}`"
                ),
            });
        }
    }

    Ok(())
}
