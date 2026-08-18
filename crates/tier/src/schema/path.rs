use std::collections::BTreeSet;

use serde_json::Value;

use crate::path::{parse_array_index_segment, path_segments};

use super::core::inlined_schema_ref;

const COMBINATOR_KEYWORDS: [&str; 3] = ["allOf", "anyOf", "oneOf"];

pub(crate) fn schema_path_explicit_array_segments(schema: &Value, path: &str) -> BTreeSet<usize> {
    let segments = path_segments(path);
    let mut explicit_array_segments = BTreeSet::new();
    let mut visited_refs = BTreeSet::new();
    collect_schema_path_explicit_array_segments(
        schema,
        schema,
        &segments,
        0,
        &mut explicit_array_segments,
        &mut visited_refs,
    );
    explicit_array_segments
}

fn collect_schema_path_explicit_array_segments(
    node: &Value,
    root: &Value,
    segments: &[&str],
    depth: usize,
    explicit_array_segments: &mut BTreeSet<usize>,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    if segments.is_empty() {
        return true;
    }

    if let Some(reference) = node.get("$ref").and_then(Value::as_str) {
        if !visited_refs.insert(reference.to_owned()) {
            return false;
        }
        let mut candidate_explicit_array_segments = explicit_array_segments.clone();
        let matched = inlined_schema_ref(node, root).is_some_and(|inlined| {
            collect_schema_path_explicit_array_segments(
                &inlined,
                root,
                segments,
                depth,
                &mut candidate_explicit_array_segments,
                visited_refs,
            )
        });
        if matched {
            *explicit_array_segments = candidate_explicit_array_segments;
        }
        visited_refs.remove(reference);
        return matched;
    }

    let Some(object) = node.as_object() else {
        return false;
    };

    let segment = segments[0];
    let remaining = &segments[1..];
    let mut matched = false;

    if let Some(child) = object
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get(segment))
        && collect_schema_path_branch(
            child,
            root,
            remaining,
            depth + 1,
            explicit_array_segments,
            visited_refs,
        )
    {
        matched = true;
    }

    for keyword in COMBINATOR_KEYWORDS {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            for child in children {
                if collect_schema_path_branch(
                    child,
                    root,
                    segments,
                    depth,
                    explicit_array_segments,
                    visited_refs,
                ) {
                    matched = true;
                }
            }
        }
    }

    if matched {
        return true;
    }

    if collect_dynamic_object_path_segment(
        object,
        root,
        segment,
        remaining,
        depth,
        explicit_array_segments,
        visited_refs,
    ) {
        return true;
    }

    collect_array_path_segment(
        object,
        root,
        segment,
        remaining,
        depth,
        explicit_array_segments,
        visited_refs,
    )
}

fn collect_dynamic_object_path_segment(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    segment: &str,
    remaining: &[&str],
    depth: usize,
    explicit_array_segments: &mut BTreeSet<usize>,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    if segment.is_empty() {
        return false;
    }

    if let Some(pattern_properties) = object.get("patternProperties").and_then(Value::as_object) {
        let mut matched = false;
        for child in pattern_properties.values() {
            if collect_schema_path_branch(
                child,
                root,
                remaining,
                depth + 1,
                explicit_array_segments,
                visited_refs,
            ) {
                matched = true;
            }
        }
        if matched {
            return true;
        }
    }

    if let Some(additional) = object
        .get("additionalProperties")
        .filter(|value| value.is_object())
        && collect_schema_path_branch(
            additional,
            root,
            remaining,
            depth + 1,
            explicit_array_segments,
            visited_refs,
        )
    {
        return true;
    }

    false
}

fn collect_array_path_segment(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    segment: &str,
    remaining: &[&str],
    depth: usize,
    explicit_array_segments: &mut BTreeSet<usize>,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let mut matched = false;
    if let Ok(index) = parse_array_index_segment(segment) {
        matched |= collect_tuple_path_segment(
            object,
            root,
            index,
            remaining,
            depth,
            explicit_array_segments,
            visited_refs,
        );
    }

    if segment == "*" {
        matched |= collect_homogeneous_array_path_segment(
            object,
            root,
            remaining,
            depth,
            explicit_array_segments,
            visited_refs,
        );
    }

    matched
}

fn collect_tuple_path_segment(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    index: usize,
    remaining: &[&str],
    depth: usize,
    explicit_array_segments: &mut BTreeSet<usize>,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let mut matched = false;
    for keyword in ["prefixItems", "items"] {
        if let Some(child) = object
            .get(keyword)
            .and_then(Value::as_array)
            .and_then(|items| items.get(index))
        {
            let mut candidate_explicit_array_segments = explicit_array_segments.clone();
            candidate_explicit_array_segments.insert(depth);
            if collect_schema_path_explicit_array_segments(
                child,
                root,
                remaining,
                depth + 1,
                &mut candidate_explicit_array_segments,
                visited_refs,
            ) {
                *explicit_array_segments = candidate_explicit_array_segments;
                matched = true;
            }
        }
    }

    matched
}

fn collect_homogeneous_array_path_segment(
    object: &serde_json::Map<String, Value>,
    root: &Value,
    remaining: &[&str],
    depth: usize,
    explicit_array_segments: &mut BTreeSet<usize>,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let mut matched = false;
    for keyword in ["items", "additionalItems", "contains"] {
        if let Some(child) = object.get(keyword).filter(|value| value.is_object()) {
            let mut candidate_explicit_array_segments = explicit_array_segments.clone();
            candidate_explicit_array_segments.insert(depth);
            if collect_schema_path_explicit_array_segments(
                child,
                root,
                remaining,
                depth + 1,
                &mut candidate_explicit_array_segments,
                visited_refs,
            ) {
                *explicit_array_segments = candidate_explicit_array_segments;
                matched = true;
            }
        }
    }

    matched
}

fn collect_schema_path_branch(
    node: &Value,
    root: &Value,
    segments: &[&str],
    depth: usize,
    explicit_array_segments: &mut BTreeSet<usize>,
    visited_refs: &mut BTreeSet<String>,
) -> bool {
    let mut candidate_explicit_array_segments = explicit_array_segments.clone();
    if collect_schema_path_explicit_array_segments(
        node,
        root,
        segments,
        depth,
        &mut candidate_explicit_array_segments,
        visited_refs,
    ) {
        *explicit_array_segments = candidate_explicit_array_segments;
        true
    } else {
        false
    }
}
