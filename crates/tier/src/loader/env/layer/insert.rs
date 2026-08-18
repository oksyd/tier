use std::collections::BTreeSet;

use crate::loader::de::insert_path_with_shape_and_explicit_arrays;
use crate::loader::overrides::{ParsedOverride, parse_env_override_value};
use crate::loader::path::{
    ensure_path_safe_keys, indexed_array_container_paths, record_direct_array_state,
    record_indexed_array_state, try_normalize_external_path_with_explicit_arrays,
};
use crate::loader::trace::record_layer_entry_traces;
use crate::path::join_path;
use crate::{ConfigError, EnvDecoder};

use super::context::EnvLayerContext;
use crate::loader::env::decoder::{custom_env_decoder_for_path, env_decoder_for_path};
use crate::loader::env::state::EnvLayerState;
use crate::loader::env::target::{
    canonicalize_runtime_env_target_path_with_explicit_arrays, claim_env_path,
};

pub(super) struct EnvInsertTarget<'a> {
    pub(super) path: &'a str,
    pub(super) external_path: Option<String>,
    pub(super) explicit_array_segments: Option<&'a BTreeSet<usize>>,
}

pub(super) fn insert_env_value(
    context: &EnvLayerContext<'_>,
    state: &mut EnvLayerState,
    name: &str,
    raw_value: &str,
    target: EnvInsertTarget<'_>,
    decoder: Option<EnvDecoder>,
) -> Result<(), ConfigError> {
    let original_path = target
        .external_path
        .unwrap_or_else(|| target.path.to_owned());
    let (_, parsed_explicit_array_segments) =
        try_normalize_external_path_with_explicit_arrays(&original_path).map_err(|message| {
            ConfigError::InvalidEnv {
                name: name.to_owned(),
                path: original_path.clone(),
                message,
            }
        })?;
    let explicit_array_segments = target
        .explicit_array_segments
        .unwrap_or(&parsed_explicit_array_segments);
    let (path, explicit_array_segments) =
        canonicalize_runtime_env_target_path_with_explicit_arrays(
            name,
            &original_path,
            explicit_array_segments,
            context.metadata,
            context.runtime_layers,
            &state.root,
            context.runtime_shape,
        )?;
    if path.is_empty() {
        return Ok(());
    }

    claim_env_path(name, &path, &mut state.claimed_paths)?;
    let parsed = decode_env_value(context, name, raw_value, &path, decoder)?;
    let is_direct_array = parsed.value.is_array();
    let segments = path.split('.').collect::<Vec<_>>();

    record_array_state(state, &path, &segments, is_direct_array, &parsed.value);
    insert_path_with_shape_and_explicit_arrays(
        &mut state.root,
        Some(context.runtime_shape),
        &segments,
        &explicit_array_segments,
        parsed.value,
    )
    .map_err(|message| ConfigError::InvalidEnv {
        name: name.to_owned(),
        path: path.clone(),
        message,
    })?;
    record_insert_metadata(
        state,
        name,
        &path,
        &segments,
        is_direct_array,
        parsed.string_coercion_suffixes,
    );

    Ok(())
}

fn decode_env_value(
    context: &EnvLayerContext<'_>,
    name: &str,
    raw_value: &str,
    path: &str,
    decoder: Option<EnvDecoder>,
) -> Result<ParsedOverride, ConfigError> {
    let custom_decoder = decoder
        .is_none()
        .then(|| custom_env_decoder_for_path(path, context.custom_env_decoders))
        .flatten();
    let metadata_env_decoder = context
        .metadata
        .effective_field_for(path)
        .and_then(|field| field.env_decode);
    let decoder = decoder
        .or_else(|| env_decoder_for_path(path, context.env_decoders))
        .or(metadata_env_decoder);

    let parsed =
        parse_env_override_value(raw_value, decoder, custom_decoder).map_err(|message| {
            ConfigError::InvalidEnv {
                name: name.to_owned(),
                path: path.to_owned(),
                message,
            }
        })?;
    ensure_path_safe_keys(&parsed.value, path).map_err(|error| match error {
        ConfigError::InvalidPathKey { path, key, message } => ConfigError::InvalidEnv {
            name: name.to_owned(),
            path,
            message: format!(
                "decoded environment value contains unsupported object key `{key}`: {message}"
            ),
        },
        _ => error,
    })?;

    Ok(parsed)
}

fn record_array_state(
    state: &mut EnvLayerState,
    path: &str,
    segments: &[&str],
    is_direct_array: bool,
    value: &serde_json::Value,
) {
    record_indexed_array_state(
        &mut state.current_array_lengths,
        &mut state.indexed_array_base_lengths,
        path,
        segments,
    );
    if is_direct_array {
        record_direct_array_state(
            &mut state.current_array_lengths,
            &mut state.indexed_array_base_lengths,
            path,
            value,
        );
    }
}

fn record_insert_metadata(
    state: &mut EnvLayerState,
    name: &str,
    path: &str,
    segments: &[&str],
    is_direct_array: bool,
    string_coercion_suffixes: std::collections::BTreeSet<String>,
) {
    for suffix in string_coercion_suffixes {
        state.coercible_string_paths.insert(if suffix.is_empty() {
            path.to_owned()
        } else {
            join_path(path, &suffix)
        });
    }
    state
        .indexed_array_paths
        .extend(indexed_array_container_paths(segments));
    if is_direct_array {
        state.direct_array_paths.insert(path.to_owned());
    }

    record_layer_entry_traces(
        &mut state.entries,
        crate::loader::SourceKind::Environment,
        "environment",
        name,
        path,
        segments,
    );
}
