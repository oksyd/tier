use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Formatter};

use serde::Serialize;
use serde_json::Value;

use crate::TierPatch;
use crate::error::ConfigError;
use crate::patch::DeferredPatchLayer;
use crate::path::collect_paths;

use super::merge::ensure_root_object;
use super::path::ensure_path_safe_keys;
use super::{SourceKind, SourceTrace};

#[derive(Clone)]
/// Custom serializable configuration layer.
pub struct Layer {
    pub(super) trace: SourceTrace,
    pub(super) value: Value,
    pub(super) entries: BTreeMap<String, SourceTrace>,
    pub(super) coercible_string_paths: BTreeSet<String>,
    pub(super) indexed_array_paths: BTreeSet<String>,
    pub(super) indexed_array_base_lengths: BTreeMap<String, usize>,
    pub(super) direct_array_paths: BTreeSet<String>,
}

impl Debug for Layer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Layer")
            .field("trace", &self.trace)
            .field("entry_count", &self.entries.len())
            .finish_non_exhaustive()
    }
}

impl Layer {
    /// Creates a custom layer from a serializable value.
    pub fn custom<T>(name: impl Into<String>, value: T) -> Result<Self, ConfigError>
    where
        T: Serialize,
    {
        Self::from_serializable(SourceTrace::new(SourceKind::Custom, name), value)
    }

    pub(super) fn from_serializable<T>(trace: SourceTrace, value: T) -> Result<Self, ConfigError>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(value)?;
        Self::from_value(trace, value)
    }

    pub(super) fn from_value(trace: SourceTrace, value: Value) -> Result<Self, ConfigError> {
        ensure_root_object(&value)?;
        ensure_path_safe_keys(&value, "")?;

        let mut paths = Vec::new();
        collect_paths(&value, "", &mut paths);
        let entries = paths
            .into_iter()
            .map(|path| (path, trace.clone()))
            .collect::<BTreeMap<_, _>>();

        Ok(Self {
            trace,
            value,
            entries,
            coercible_string_paths: BTreeSet::new(),
            indexed_array_paths: BTreeSet::new(),
            indexed_array_base_lengths: BTreeMap::new(),
            direct_array_paths: BTreeSet::new(),
        })
    }

    pub(crate) fn from_parts(
        trace: SourceTrace,
        value: Value,
        entries: BTreeMap<String, SourceTrace>,
        coercible_string_paths: BTreeSet<String>,
        indexed_array_paths: BTreeSet<String>,
        indexed_array_base_lengths: BTreeMap<String, usize>,
        direct_array_paths: BTreeSet<String>,
    ) -> Self {
        Self {
            trace,
            value,
            entries,
            coercible_string_paths,
            indexed_array_paths,
            indexed_array_base_lengths,
            direct_array_paths,
        }
    }

    /// Creates a custom configuration layer from a typed sparse patch.
    ///
    /// This is the typed alternative to manually building a [`Layer`] from a
    /// serializable shadow struct.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "derive")] {
    /// # fn main() -> Result<(), tier::ConfigError> {
    /// use tier::{Layer, TierPatch};
    ///
    /// #[derive(Debug, TierPatch, Default)]
    /// struct CliPatch {
    ///     port: Option<u16>,
    /// }
    ///
    /// let _layer = Layer::from_patch("typed-cli", &CliPatch { port: Some(7000) })?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn from_patch<P>(name: impl Into<String>, patch: &P) -> Result<Self, ConfigError>
    where
        P: TierPatch,
    {
        Self::from_patch_with_trace(
            SourceTrace {
                kind: SourceKind::Custom,
                name: name.into(),
                location: None,
            },
            patch,
        )
    }

    pub(crate) fn from_patch_with_trace<P>(
        trace: SourceTrace,
        patch: &P,
    ) -> Result<Self, ConfigError>
    where
        P: TierPatch,
    {
        let mut builder = crate::patch::PatchLayerBuilder::from_trace(trace);
        patch.write_layer(&mut builder, "")?;
        Ok(builder.finish())
    }
}

pub(super) enum PendingCustomLayer {
    Immediate(Layer),
    DeferredPatch(DeferredPatchLayer),
}
