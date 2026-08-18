use std::fmt::{self, Display, Formatter};

use serde_json::Value;

use crate::loader::SourceTrace;

use super::super::render::render_value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// One source contribution recorded for a configuration path.
pub struct ResolutionStep {
    /// Source that wrote the value.
    pub source: SourceTrace,
    /// Value contributed by the source.
    pub value: Value,
    /// Whether the recorded value was redacted.
    pub redacted: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Full resolution trace for a single configuration path.
pub struct Explanation {
    /// Dot-delimited configuration path.
    pub path: String,
    /// Final value for the path after all layers and normalization.
    pub final_value: Option<Value>,
    /// Ordered source contributions for the path.
    pub steps: Vec<ResolutionStep>,
    /// Whether the path is considered sensitive.
    pub redacted: bool,
}

impl Display for Explanation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let final_value = self
            .final_value
            .as_ref()
            .map_or_else(|| "null".to_owned(), render_value);
        writeln!(f, "{} = {}", self.path, final_value)?;

        for step in &self.steps {
            write!(f, "- {}", step.source)?;
            if !step.source.name.is_empty() {
                write!(f, ": ")?;
            } else {
                write!(f, " ")?;
            }
            writeln!(f, "{}", render_value(&step.value))?;
        }

        Ok(())
    }
}
