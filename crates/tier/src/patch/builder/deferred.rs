use serde_json::Value;

use crate::{ConfigError, Layer, SourceTrace};

use super::model::PatchLayerBuilder;

pub(crate) struct DeferredPatchLayer {
    trace: SourceTrace,
    writes: Vec<(String, Value)>,
}

impl DeferredPatchLayer {
    pub(super) fn new(trace: SourceTrace, writes: Vec<(String, Value)>) -> Self {
        Self { trace, writes }
    }

    pub(crate) fn into_layer_with_shape(self, shape: Value) -> Result<Layer, ConfigError> {
        let mut builder = PatchLayerBuilder::from_trace_with_shape(self.trace, shape);
        for (path, value) in self.writes {
            builder.insert_value(&path, value)?;
        }
        Ok(builder.finish())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
}
