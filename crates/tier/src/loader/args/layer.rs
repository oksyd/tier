use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::ConfigError;
use crate::path::join_path;

use super::super::de::insert_path_with_shape_and_explicit_arrays;
use super::super::overrides::ParsedOverride;
use super::super::path::{
    ensure_path_safe_keys, indexed_array_container_paths, invalid_concrete_path_segment,
    record_direct_array_state, record_indexed_array_state,
};
use super::super::trace::record_layer_entry_traces;
use super::super::{Layer, SourceKind, SourceTrace};
use super::claim::claim_arg_path;

pub(in crate::loader) struct ArgsLayerState {
    root: Value,
    entries: BTreeMap<String, SourceTrace>,
    coercible_string_paths: BTreeSet<String>,
    indexed_array_paths: BTreeSet<String>,
    indexed_array_base_lengths: BTreeMap<String, usize>,
    current_array_lengths: BTreeMap<String, usize>,
    direct_array_paths: BTreeSet<String>,
    claimed_paths: BTreeMap<String, String>,
}

impl ArgsLayerState {
    pub(in crate::loader) fn new() -> Self {
        Self {
            root: Value::Object(Map::new()),
            entries: BTreeMap::new(),
            coercible_string_paths: BTreeSet::new(),
            indexed_array_paths: BTreeSet::new(),
            indexed_array_base_lengths: BTreeMap::new(),
            current_array_lengths: BTreeMap::new(),
            direct_array_paths: BTreeSet::new(),
            claimed_paths: BTreeMap::new(),
        }
    }

    pub(in crate::loader) fn insert_override(
        &mut self,
        source_name: &str,
        path: &str,
        parsed: ParsedOverride,
        error_arg: String,
        shape: Option<&Value>,
        explicit_array_segments: &BTreeSet<usize>,
    ) -> Result<(), ConfigError> {
        let ParsedOverride {
            value,
            string_coercion_suffixes,
        } = parsed;
        validate_arg_path(path, &error_arg)?;
        validate_arg_value_keys(&value, &error_arg, path)?;

        let is_direct_array = value.is_array();
        claim_arg_path(
            source_name,
            path,
            is_direct_array,
            &self.direct_array_paths,
            &mut self.claimed_paths,
        )?;

        let segments = path.split('.').collect::<Vec<_>>();
        record_indexed_array_state(
            &mut self.current_array_lengths,
            &mut self.indexed_array_base_lengths,
            path,
            &segments,
        );
        if is_direct_array {
            record_direct_array_state(
                &mut self.current_array_lengths,
                &mut self.indexed_array_base_lengths,
                path,
                &value,
            );
        }

        insert_path_with_shape_and_explicit_arrays(
            &mut self.root,
            shape,
            &segments,
            explicit_array_segments,
            value,
        )
        .map_err(|message| ConfigError::InvalidArg {
            arg: error_arg,
            message,
        })?;
        for suffix in string_coercion_suffixes {
            self.coercible_string_paths.insert(if suffix.is_empty() {
                path.to_owned()
            } else {
                join_path(path, &suffix)
            });
        }
        self.indexed_array_paths
            .extend(indexed_array_container_paths(&segments));
        if is_direct_array {
            self.direct_array_paths.insert(path.to_owned());
        }

        record_layer_entry_traces(
            &mut self.entries,
            SourceKind::Arguments,
            "arguments",
            source_name,
            path,
            &segments,
        );
        Ok(())
    }

    pub(in crate::loader) fn into_layer(self) -> Option<Layer> {
        if self.entries.is_empty() {
            return None;
        }

        Some(Layer {
            trace: SourceTrace::new(SourceKind::Arguments, "arguments"),
            value: self.root,
            entries: self.entries,
            coercible_string_paths: self.coercible_string_paths,
            indexed_array_paths: self.indexed_array_paths,
            indexed_array_base_lengths: self.indexed_array_base_lengths,
            direct_array_paths: self.direct_array_paths,
        })
    }
}

fn validate_arg_path(path: &str, error_arg: &str) -> Result<(), ConfigError> {
    if let Some((segment, message)) = invalid_concrete_path_segment(path) {
        return Err(ConfigError::InvalidArg {
            arg: error_arg.to_owned(),
            message: format!("configuration path segment `{segment}` is invalid: {message}"),
        });
    }

    Ok(())
}

fn validate_arg_value_keys(value: &Value, error_arg: &str, path: &str) -> Result<(), ConfigError> {
    ensure_path_safe_keys(value, path).map_err(|error| match error {
        ConfigError::InvalidPathKey { path, key, message } => ConfigError::InvalidArg {
            arg: error_arg.to_owned(),
            message: format!(
                "override value at `{path}` contains unsupported object key `{key}`: {message}"
            ),
        },
        _ => error,
    })
}
