use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::super::{Layer, SourceKind, SourceTrace};

pub(super) struct EnvLayerState {
    pub(super) root: Value,
    pub(super) entries: BTreeMap<String, SourceTrace>,
    pub(super) coercible_string_paths: BTreeSet<String>,
    pub(super) indexed_array_paths: BTreeSet<String>,
    pub(super) indexed_array_base_lengths: BTreeMap<String, usize>,
    pub(super) current_array_lengths: BTreeMap<String, usize>,
    pub(super) direct_array_paths: BTreeSet<String>,
    pub(super) claimed_paths: BTreeMap<String, String>,
}

impl EnvLayerState {
    pub(super) fn new() -> Self {
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

    pub(super) fn into_layer(self) -> Option<Layer> {
        if self.entries.is_empty() {
            return None;
        }

        Some(Layer {
            trace: SourceTrace::new(SourceKind::Environment, "environment"),
            value: self.root,
            entries: self.entries,
            coercible_string_paths: self.coercible_string_paths,
            indexed_array_paths: self.indexed_array_paths,
            indexed_array_base_lengths: self.indexed_array_base_lengths,
            direct_array_paths: self.direct_array_paths,
        })
    }
}
