use serde_json::Value;

use crate::loader::{CustomEnvDecoder, Layer};
use crate::{ConfigMetadata, EnvDecoder};
use std::collections::BTreeMap;

pub(super) struct EnvLayerContext<'a> {
    pub(super) metadata: &'a ConfigMetadata,
    pub(super) env_decoders: &'a BTreeMap<String, EnvDecoder>,
    pub(super) custom_env_decoders: &'a BTreeMap<String, CustomEnvDecoder>,
    pub(super) runtime_layers: &'a [Layer],
    pub(super) runtime_shape: &'a Value,
}
