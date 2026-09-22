#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde_json::json;
use tier::{ConfigLoader, Layer};

#[test]
fn historical_object_paths_remain_explainable_after_array_replacement() {
    let loaded = ConfigLoader::new(json!({"items": {"old": "historical"}}))
        .layer(Layer::custom("replacement", json!({"items": ["new"]})).unwrap())
        .load()
        .unwrap();
    let explanation = loaded.report().explain("items.old").unwrap();
    assert!(explanation.final_value.is_none());
    assert_eq!(explanation.steps.len(), 1);
    assert_eq!(explanation.steps[0].value, "historical");
    let audit = loaded.report().audit_report();
    assert_eq!(audit.summary.trace_count, audit.traces.len());
    assert!(audit.traces.contains_key("items.old"));
}

#[test]
fn audit_retains_exact_historical_numeric_object_keys() {
    let loaded = ConfigLoader::new(json!({"items": {"00": "historical"}}))
        .layer(Layer::custom("replacement", json!({"items": ["current"]})).unwrap())
        .load()
        .unwrap();
    let audit = loaded.report().audit_report();
    let historical = &audit.traces["items.00"].explanation;
    assert_eq!(historical.path, "items.00");
    assert_eq!(historical.steps[0].value, "historical");
    assert!(historical.final_value.is_none());
    assert_eq!(
        audit.traces["items.0"].explanation.steps[0].value,
        "current"
    );
}
