use serde_json::Value;

use crate::ConfigMetadata;

mod comments;
mod path;
mod render;
mod value;

pub(super) fn render_example_toml(value: &Value, metadata: &ConfigMetadata) -> String {
    render::render_example_toml(value, metadata)
}
