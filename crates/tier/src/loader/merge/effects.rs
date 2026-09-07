use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::path::{join_path, path_is_at_or_below};

use super::super::{Layer, SourceTrace};

/// Bookkeeping produced by the same traversal that merges the values.
#[derive(Default)]
pub(in crate::loader) struct MergeEffects {
    pub(super) replaced_paths: BTreeSet<String>,
    pub(super) append_offsets: BTreeMap<String, usize>,
    pub(in crate::loader) removed_entries: BTreeMap<String, SourceTrace>,
}

impl MergeEffects {
    pub(super) fn record_removals(
        &mut self,
        before: &Value,
        after: Option<&Value>,
        path: &str,
        trace: &SourceTrace,
    ) {
        if after.is_none() {
            self.removed_entries.insert(path.to_owned(), trace.clone());
        }
        match before {
            Value::Object(values) => {
                for (key, value) in values {
                    self.record_removals(
                        value,
                        after
                            .and_then(Value::as_object)
                            .and_then(|map| map.get(key)),
                        &join_path(path, key),
                        trace,
                    );
                }
            }
            Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    self.record_removals(
                        value,
                        after
                            .and_then(Value::as_array)
                            .and_then(|array| array.get(index)),
                        &join_path(path, &index.to_string()),
                        trace,
                    );
                }
            }
            _ => {}
        }
    }

    pub(in crate::loader) fn update_coercion_paths(
        &self,
        paths: &mut BTreeSet<String>,
        layer: &Layer,
    ) {
        paths.retain(|path| {
            !self
                .replaced_paths
                .iter()
                .any(|replaced| path_is_at_or_below(path, replaced))
        });
        paths.extend(
            layer
                .coercible_string_paths
                .iter()
                .map(|path| self.destination_path(path)),
        );
    }

    /// Align trace values and paths with their destination indices. Leading nulls
    /// are placeholders, not contributions from this layer.
    pub(in crate::loader) fn align_trace_layer(&self, layer: &mut Layer) {
        if self.append_offsets.is_empty() {
            return;
        }
        layer.value = self.align_value(std::mem::take(&mut layer.value), "");
        layer.entries = std::mem::take(&mut layer.entries)
            .into_iter()
            .map(|(path, trace)| (self.destination_path(&path), trace))
            .collect();
    }

    pub(in crate::loader) fn destination_path(&self, path: &str) -> String {
        let mut source = String::new();
        let mut destination = String::new();
        for segment in path.split('.') {
            let mapped = self.append_offsets.get(&source).and_then(|offset| {
                segment
                    .parse::<usize>()
                    .ok()
                    .map(|index| (offset + index).to_string())
            });
            destination = join_path(&destination, mapped.as_deref().unwrap_or(segment));
            source = join_path(&source, segment);
        }
        destination
    }

    fn align_value(&self, value: Value, path: &str) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(key, value)| {
                        let value = self.align_value(value, &join_path(path, &key));
                        (key, value)
                    })
                    .collect(),
            ),
            Value::Array(values) => {
                let mut aligned =
                    vec![Value::Null; self.append_offsets.get(path).copied().unwrap_or(0)];
                aligned.extend(values.into_iter().enumerate().map(|(index, value)| {
                    self.align_value(value, &join_path(path, &index.to_string()))
                }));
                Value::Array(aligned)
            }
            value => value,
        }
    }
}
