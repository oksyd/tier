use serde_json::Value;

use crate::path::{checked_array_len_for_index, join_path};
use crate::{ConfigError, ConfigMetadata, MergeStrategy};

use super::{Layer, SourceKind};

mod effects;
pub(super) use effects::MergeEffects;

pub(crate) fn ensure_root_object(value: &Value) -> Result<(), ConfigError> {
    if matches!(value, Value::Object(_)) {
        Ok(())
    } else {
        Err(ConfigError::RootMustBeObject {
            actual: value_kind(value),
        })
    }
}

pub(crate) fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub(super) fn merged_shape_from_layers(
    defaults: &Value,
    layers: &[Layer],
    metadata: &ConfigMetadata,
) -> Result<Value, ConfigError> {
    // Once collected, the default layer also carries canonicalized alias paths.
    let mut shape = layers
        .iter()
        .find(|layer| matches!(layer.trace.kind, SourceKind::Default))
        .map_or(defaults, |layer| &layer.value)
        .clone();
    ensure_root_object(&shape)?;
    for layer in layers {
        if !matches!(layer.trace.kind, SourceKind::Default) {
            merge_layer(&mut shape, layer, metadata)?;
        }
    }
    Ok(shape)
}

pub(super) fn merge_layer(
    target: &mut Value,
    layer: &Layer,
    metadata: &ConfigMetadata,
) -> Result<MergeEffects, ConfigError> {
    let mut context = MergeContext {
        layer,
        metadata,
        effects: MergeEffects::default(),
    };
    context.merge(target, &layer.value, "")?;
    Ok(context.effects)
}

struct MergeContext<'a> {
    layer: &'a Layer,
    metadata: &'a ConfigMetadata,
    effects: MergeEffects,
}

impl MergeContext<'_> {
    fn merge(
        &mut self,
        target: &mut Value,
        overlay: &Value,
        path: &str,
    ) -> Result<(), ConfigError> {
        let strategy = self
            .metadata
            .merge_strategy_for(path)
            .unwrap_or(MergeStrategy::Merge);
        let indexed_patch = self.layer.indexed_array_paths.contains(path)
            && !self.layer.direct_array_paths.contains(path);
        match (target, overlay) {
            (Value::Array(target), Value::Array(overlay)) if indexed_patch && !path.is_empty() => {
                for (index, value) in overlay.iter().enumerate() {
                    let child_path = join_path(path, &index.to_string());
                    // Sparse placeholders may be null or empty containers. Only
                    // claimed indices contribute, including explicit null writes.
                    if !self.layer.entries.contains_key(&child_path) {
                        continue;
                    }
                    if target.len() <= index {
                        target.resize(
                            checked_array_len_for_index(index).map_err(|message| {
                                ConfigError::MetadataInvalid {
                                    path: child_path.clone(),
                                    message,
                                }
                            })?,
                            Value::Null,
                        );
                    }
                    self.merge(&mut target[index], value, &child_path)?;
                }
            }
            (target, overlay) if strategy == MergeStrategy::Replace => {
                self.replace(target, overlay, path);
            }
            (Value::Array(target), Value::Array(overlay)) if strategy == MergeStrategy::Append => {
                self.effects
                    .append_offsets
                    .insert(path.to_owned(), target.len());
                target.extend(overlay.iter().cloned());
            }
            (Value::Object(target), Value::Object(overlay)) => {
                for (key, value) in overlay {
                    let child_path = join_path(path, key);
                    if let Some(existing) = target.get_mut(key) {
                        self.merge(existing, value, &child_path)?;
                    } else {
                        self.effects.replaced_paths.insert(child_path);
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
            (target, overlay) => self.replace(target, overlay, path),
        }
        Ok(())
    }

    fn replace(&mut self, target: &mut Value, overlay: &Value, path: &str) {
        let trace = self.layer.entries.get(path).unwrap_or(&self.layer.trace);
        self.effects
            .record_removals(target, Some(overlay), path, trace);
        self.effects.replaced_paths.insert(path.to_owned());
        *target = overlay.clone();
    }
}
