use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::{Layer, SourceKind, SourceTrace};

use super::deferred::DeferredPatchLayer;

/// Hidden helper used by the `TierPatch` derive macro.
#[doc(hidden)]
pub struct PatchLayerBuilder {
    pub(super) trace: SourceTrace,
    pub(super) value: Value,
    pub(super) shape: Value,
    pub(super) entries: BTreeMap<String, SourceTrace>,
    pub(super) claimed_paths: BTreeSet<String>,
    pub(super) indexed_array_paths: BTreeSet<String>,
    pub(super) indexed_array_base_lengths: BTreeMap<String, usize>,
    pub(super) current_array_lengths: BTreeMap<String, usize>,
    pub(super) direct_array_paths: BTreeSet<String>,
    pub(super) deferred_writes: Option<Vec<(String, Value)>>,
}

impl PatchLayerBuilder {
    /// Creates a builder for a synthetic patch layer.
    #[must_use]
    pub fn new(kind: SourceKind, name: impl Into<String>) -> Self {
        Self::from_trace(SourceTrace {
            kind,
            name: name.into(),
            location: None,
        })
    }

    /// Creates a builder from an explicit source trace.
    #[must_use]
    pub fn from_trace(trace: SourceTrace) -> Self {
        Self::from_trace_with_shape(trace, Value::Object(Map::new()))
    }

    #[must_use]
    pub(crate) fn from_trace_deferred(trace: SourceTrace) -> Self {
        Self {
            trace,
            value: Value::Object(Map::new()),
            shape: Value::Object(Map::new()),
            entries: BTreeMap::new(),
            claimed_paths: BTreeSet::new(),
            indexed_array_paths: BTreeSet::new(),
            indexed_array_base_lengths: BTreeMap::new(),
            current_array_lengths: BTreeMap::new(),
            direct_array_paths: BTreeSet::new(),
            deferred_writes: Some(Vec::new()),
        }
    }

    /// Creates a builder from an explicit source trace and an existing shape.
    #[must_use]
    pub fn from_trace_with_shape(trace: SourceTrace, shape: Value) -> Self {
        Self {
            trace,
            value: Value::Object(Map::new()),
            shape,
            entries: BTreeMap::new(),
            claimed_paths: BTreeSet::new(),
            indexed_array_paths: BTreeSet::new(),
            indexed_array_base_lengths: BTreeMap::new(),
            current_array_lengths: BTreeMap::new(),
            direct_array_paths: BTreeSet::new(),
            deferred_writes: None,
        }
    }

    /// Finalizes the builder into a [`Layer`].
    #[must_use]
    pub fn finish(self) -> Layer {
        Layer::from_parts(
            self.trace,
            self.value,
            self.entries,
            BTreeSet::new(),
            self.indexed_array_paths,
            self.indexed_array_base_lengths,
            self.direct_array_paths,
        )
    }

    pub(crate) fn finish_deferred(self) -> DeferredPatchLayer {
        DeferredPatchLayer::new(self.trace, self.deferred_writes.unwrap_or_default())
    }
}
