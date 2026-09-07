use std::collections::BTreeMap;

use serde_json::Value;

use crate::path::join_path;

use super::{Layer, SourceTrace};

impl Layer {
    /// Expand structured writes before policy checks and trace recording. Sparse
    /// array placeholders are not writes and must not inherit the parent source.
    pub(in crate::loader) fn expand_entry_traces(&mut self) {
        let mut entries = BTreeMap::new();
        self.collect_entry_traces(&self.value, "", &self.trace, &mut entries);
        self.entries = entries;
    }

    fn collect_entry_traces(
        &self,
        value: &Value,
        path: &str,
        inherited: &SourceTrace,
        entries: &mut BTreeMap<String, SourceTrace>,
    ) {
        let trace = self.entries.get(path).unwrap_or(inherited);
        if !path.is_empty() {
            entries.insert(path.to_owned(), trace.clone());
        }
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    self.collect_entry_traces(value, &join_path(path, key), trace, entries);
                }
            }
            Value::Array(values) => {
                let sparse = self.indexed_array_paths.contains(path)
                    && !self.direct_array_paths.contains(path);
                for (index, value) in values.iter().enumerate() {
                    let child_path = join_path(path, &index.to_string());
                    if sparse && !self.entries.contains_key(&child_path) {
                        continue;
                    }
                    self.collect_entry_traces(value, &child_path, trace, entries);
                }
            }
            _ => {}
        }
    }
}
