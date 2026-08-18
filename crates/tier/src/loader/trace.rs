use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::metadata::ConfigMetadata;
use crate::path::{
    collect_diff_paths, collect_paths, get_value_at_path, path_overlaps_pattern,
    path_starts_with_pattern, redact_value,
};
use crate::report::{ConfigReport, ConfigWarning, DeprecatedField, ResolutionStep};
use crate::value::values_equal;

use super::{Layer, SourceKind, SourceTrace};

pub(super) fn record_layer_steps(
    report: &mut ConfigReport,
    layer: &Layer,
    secret_paths: &BTreeSet<String>,
) {
    report.record_step(
        String::new(),
        ResolutionStep {
            source: layer.trace.clone(),
            value: redact_value(&layer.value, "", secret_paths),
            redacted: path_contains_secret(secret_paths, ""),
        },
    );

    for (path, trace) in &layer.entries {
        if let Some(value) = get_value_at_path(&layer.value, path) {
            let redacted = path_contains_secret(secret_paths, path);
            let rendered = redact_value(value, path, secret_paths);
            report.record_step(
                path.clone(),
                ResolutionStep {
                    source: trace.clone(),
                    value: rendered,
                    redacted,
                },
            );
        }
    }
}

pub(super) fn record_diff_steps(
    report: &mut ConfigReport,
    before: &Value,
    after: &Value,
    trace: &SourceTrace,
    secret_paths: &BTreeSet<String>,
) {
    if !values_equal(before, after) {
        report.record_step(
            String::new(),
            ResolutionStep {
                source: trace.clone(),
                value: redact_value(after, "", secret_paths),
                redacted: path_contains_secret(secret_paths, ""),
            },
        );
    }

    let mut paths = Vec::new();
    collect_diff_paths(before, after, "", &mut paths);
    paths.sort();
    paths.dedup();

    for path in paths {
        let after_value = get_value_at_path(after, &path).cloned();
        let removed = after_value.is_none() && get_value_at_path(before, &path).is_some();
        if !removed && after_value.is_none() {
            continue;
        }

        let redacted = path_contains_secret(secret_paths, &path);
        let rendered = match after_value {
            Some(value) => redact_value(&value, &path, secret_paths),
            None => Value::Null,
        };
        report.record_step(
            path,
            ResolutionStep {
                source: trace.clone(),
                value: rendered,
                redacted,
            },
        );
    }
}

pub(in crate::loader) fn record_layer_entry_traces(
    entries: &mut BTreeMap<String, SourceTrace>,
    kind: SourceKind,
    aggregate_name: &str,
    exact_name: &str,
    path: &str,
    segments: &[&str],
) {
    entries.insert(
        path.to_owned(),
        SourceTrace::new(kind, exact_name.to_owned()),
    );

    let mut prefix = String::new();
    for segment in segments {
        if !prefix.is_empty() {
            prefix.push('.');
        }
        prefix.push_str(segment);
        let entry = entries
            .entry(prefix.clone())
            .or_insert_with(|| SourceTrace::new(kind, exact_name.to_owned()));
        if prefix != path && entry.name != exact_name {
            *entry = SourceTrace::new(kind, aggregate_name.to_owned());
        }
    }
}

pub(super) fn record_deprecation_warnings(
    report: &mut ConfigReport,
    layer: &Layer,
    metadata: &ConfigMetadata,
) {
    if matches!(layer.trace.kind, SourceKind::Default) {
        return;
    }

    let mut used_paths = Vec::new();
    collect_paths(&layer.value, "", &mut used_paths);
    used_paths.sort();
    used_paths.dedup();

    let mut warned = BTreeSet::new();
    for path in used_paths {
        let Some(field) = metadata.effective_field_for(&path) else {
            continue;
        };
        let Some(note) = field.deprecated.clone() else {
            continue;
        };
        if warned.insert(path.clone()) {
            report.record_warning(ConfigWarning::DeprecatedField(
                DeprecatedField::new(path)
                    .with_source(Some(layer.trace.clone()))
                    .with_note(Some(note)),
            ));
        }
    }
}

pub(crate) fn is_secret_path(secret_paths: &BTreeSet<String>, path: &str) -> bool {
    secret_paths
        .iter()
        .any(|secret| path_starts_with_pattern(path, secret))
}

fn path_contains_secret(secret_paths: &BTreeSet<String>, path: &str) -> bool {
    secret_paths
        .iter()
        .any(|secret| path_overlaps_pattern(path, secret))
}
