use serde_json::Value;

use crate::path::{collect_diff_paths, parse_array_index_segment};
use crate::value::values_equal;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// A single redacted configuration change observed during reload.
pub struct ConfigChange {
    /// Dot-delimited path that changed.
    pub path: String,
    /// Previous redacted value, when present.
    pub before: Option<Value>,
    /// New redacted value, when present.
    pub after: Option<Value>,
    /// Whether either side of the change was redacted.
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Structured summary of a successful reload attempt.
pub struct ReloadSummary {
    /// Whether the effective configuration changed.
    pub had_changes: bool,
    /// Changed paths in normalized order.
    pub changed_paths: Vec<String>,
    /// Structured per-path change details.
    pub changes: Vec<ConfigChange>,
}

impl ReloadSummary {
    /// Returns `true` when the reload was a no-op.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        !self.had_changes
    }
}

pub(super) fn build_reload_summary(
    before_raw: &Value,
    after_raw: &Value,
    before_redacted: &Value,
    after_redacted: &Value,
) -> ReloadSummary {
    let mut changed_paths = Vec::new();
    collect_diff_paths(before_raw, after_raw, "", &mut changed_paths);
    changed_paths.sort();
    changed_paths.dedup();

    let changes = changed_paths
        .iter()
        .map(|path| {
            let before_value = redacted_value_at_path(before_redacted, path);
            let after_value = redacted_value_at_path(after_redacted, path);
            let redacted = before_value.as_ref().is_some_and(is_redacted_value)
                || after_value.as_ref().is_some_and(is_redacted_value);
            ConfigChange {
                path: path.clone(),
                before: before_value,
                after: after_value,
                redacted,
            }
        })
        .collect::<Vec<_>>();

    ReloadSummary {
        had_changes: !values_equal(before_raw, after_raw),
        changed_paths,
        changes,
    }
}

fn redacted_value_at_path(value: &Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(value.clone());
    }

    let mut current = value;
    for segment in path.split('.') {
        match current {
            Value::String(text) if text == "***redacted***" => {
                return Some(Value::String(text.clone()));
            }
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            Value::Array(values) => {
                let index = parse_array_index_segment(segment).ok()?;
                current = values.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current.clone())
}

fn is_redacted_value(value: &Value) -> bool {
    match value {
        Value::String(text) => text == "***redacted***",
        Value::Array(values) => values.iter().any(is_redacted_value),
        Value::Object(map) => map.values().any(is_redacted_value),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::build_reload_summary;

    #[test]
    fn reload_summary_treats_numeric_values_semantically() {
        let before = json!({ "value": 1 });
        let after = json!({ "value": 1.0 });

        let summary = build_reload_summary(&before, &after, &before, &after);

        assert!(summary.is_noop());
        assert!(summary.changed_paths.is_empty());
        assert!(summary.changes.is_empty());
    }
}
