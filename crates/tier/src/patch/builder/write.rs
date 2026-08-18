use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::ConfigError;
use crate::loader::record_direct_array_state;
use crate::path::normalize_path;

use super::model::PatchLayerBuilder;
use crate::patch::path::{
    canonicalize_patch_path, parse_patch_path, patch_indexed_array_container_paths,
};
use crate::patch::write::{
    claim_patch_path, insert_path_with_shape, record_patch_indexed_array_state,
};

struct PreparedPatchWrite {
    original_path: String,
    canonical_path: String,
    segments: Vec<String>,
    array_segments: BTreeSet<usize>,
    indexed_array_container_paths: BTreeSet<String>,
}

impl PatchLayerBuilder {
    /// Inserts a serializable leaf override.
    pub fn insert_serialized<T>(&mut self, path: &str, value: &T) -> Result<(), ConfigError>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(value)?;
        self.insert_value(path, value)
    }

    /// Inserts a pre-built JSON value override.
    pub fn insert_value(&mut self, path: &str, value: Value) -> Result<(), ConfigError> {
        let shape_snapshot = self.shape.clone();
        let write = self.prepare_write(path, &shape_snapshot)?;

        record_patch_indexed_array_state(
            &mut self.current_array_lengths,
            &mut self.indexed_array_base_lengths,
            &write.canonical_path,
            &write.indexed_array_container_paths,
        );
        if value.is_array() {
            record_direct_array_state(
                &mut self.current_array_lengths,
                &mut self.indexed_array_base_lengths,
                &write.canonical_path,
                &value,
            );
        }

        if self.deferred_writes.is_some() {
            self.insert_shape(&shape_snapshot, &write, value.clone())?;
            if let Some(writes) = &mut self.deferred_writes {
                writes.push((write.original_path, value));
            }
            return Ok(());
        }

        self.insert_value_layer(&shape_snapshot, &write, value)
    }

    fn prepare_write(
        &mut self,
        path: &str,
        shape_snapshot: &Value,
    ) -> Result<PreparedPatchWrite, ConfigError> {
        let (segments, explicit_array_segments) =
            parse_patch_path(path).map_err(|message| ConfigError::InvalidPatch {
                name: self.trace.name.clone(),
                path: path.to_owned(),
                message,
            })?;
        if segments.is_empty() {
            return Err(ConfigError::InvalidPatch {
                name: self.trace.name.clone(),
                path: String::new(),
                message: "configuration path cannot be empty".to_owned(),
            });
        }

        let (segments, array_segments) =
            canonicalize_patch_path(shape_snapshot, &segments, &explicit_array_segments).map_err(
                |message| ConfigError::InvalidPatch {
                    name: self.trace.name.clone(),
                    path: path.to_owned(),
                    message,
                },
            )?;
        let canonical_path = normalize_path(&segments.join("."));
        claim_patch_path(&self.trace.name, &canonical_path, &mut self.claimed_paths)?;
        let indexed_array_container_paths =
            patch_indexed_array_container_paths(&segments, &array_segments);

        Ok(PreparedPatchWrite {
            original_path: path.to_owned(),
            canonical_path,
            segments,
            array_segments,
            indexed_array_container_paths,
        })
    }

    fn insert_value_layer(
        &mut self,
        shape_snapshot: &Value,
        write: &PreparedPatchWrite,
        value: Value,
    ) -> Result<(), ConfigError> {
        if value.is_array() {
            self.direct_array_paths.insert(write.canonical_path.clone());
        }
        insert_path_with_shape(
            &mut self.value,
            Some(shape_snapshot),
            &write.segments,
            &write.array_segments,
            0,
            value.clone(),
        )
        .map_err(|message| ConfigError::InvalidPatch {
            name: self.trace.name.clone(),
            path: write.canonical_path.clone(),
            message,
        })?;
        self.insert_shape(shape_snapshot, write, value)?;
        self.indexed_array_paths
            .extend(write.indexed_array_container_paths.iter().cloned());
        self.record_entry_prefixes(&write.segments, &write.canonical_path);
        Ok(())
    }

    fn insert_shape(
        &mut self,
        shape_snapshot: &Value,
        write: &PreparedPatchWrite,
        value: Value,
    ) -> Result<(), ConfigError> {
        insert_path_with_shape(
            &mut self.shape,
            Some(shape_snapshot),
            &write.segments,
            &write.array_segments,
            0,
            value,
        )
        .map_err(|message| ConfigError::InvalidPatch {
            name: self.trace.name.clone(),
            path: write.canonical_path.clone(),
            message,
        })
    }

    fn record_entry_prefixes(&mut self, segments: &[String], canonical_path: &str) {
        self.entries
            .insert(canonical_path.to_owned(), self.trace.clone());
        let mut prefix = String::new();
        for segment in segments {
            if !prefix.is_empty() {
                prefix.push('.');
            }
            prefix.push_str(segment);
            self.entries
                .entry(prefix.clone())
                .or_insert_with(|| self.trace.clone());
        }
    }
}
