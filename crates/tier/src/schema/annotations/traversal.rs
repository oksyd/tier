use std::collections::BTreeSet;

use serde_json::Value;

use crate::FieldMetadata;
use crate::path::parse_array_index_segment;
use crate::schema::core::inlined_schema_ref;

use super::field::annotate_schema_node;

const COMBINATOR_KEYWORDS: [&str; 3] = ["allOf", "anyOf", "oneOf"];

pub(super) fn annotate_schema_path(
    schema: &mut Value,
    root: &Value,
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    field: &FieldMetadata,
) -> bool {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    annotate_schema_segments(schema, root, &segments, explicit_array_segments, 0, field)
}

fn annotate_schema_segments(
    node: &mut Value,
    root: &Value,
    segments: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    field: &FieldMetadata,
) -> bool {
    if segments.is_empty() {
        annotate_schema_node(node, field);
        return true;
    }

    inline_schema_ref(node, root);
    let Some(object) = node.as_object_mut() else {
        return false;
    };

    let segment = segments[0];
    let remaining = &segments[1..];

    let mut matched = if segment == "*" {
        annotate_wildcard_children(
            object,
            root,
            remaining,
            explicit_array_segments,
            depth,
            field,
        )
    } else {
        annotate_named_child(
            object,
            root,
            segment,
            remaining,
            explicit_array_segments,
            depth,
            field,
        )
    };

    for keyword in COMBINATOR_KEYWORDS {
        if let Some(children) = object.get_mut(keyword).and_then(Value::as_array_mut) {
            for child in children {
                matched |= annotate_schema_segments(
                    child,
                    root,
                    segments,
                    explicit_array_segments,
                    depth,
                    field,
                );
            }
        }
    }

    matched
}

fn annotate_wildcard_children(
    object: &mut serde_json::Map<String, Value>,
    root: &Value,
    remaining: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    field: &FieldMetadata,
) -> bool {
    let mut matched = false;
    annotate_object_children(
        object,
        root,
        remaining,
        explicit_array_segments,
        depth,
        field,
        &mut matched,
    );
    annotate_array_children(
        object,
        root,
        remaining,
        explicit_array_segments,
        depth,
        field,
        &mut matched,
    );
    annotate_dynamic_children(
        object,
        root,
        remaining,
        explicit_array_segments,
        depth,
        field,
        &mut matched,
    );
    matched
}

fn annotate_object_children(
    object: &mut serde_json::Map<String, Value>,
    root: &Value,
    remaining: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    field: &FieldMetadata,
    matched: &mut bool,
) {
    if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
        for child in properties.values_mut() {
            *matched |= annotate_schema_segments(
                child,
                root,
                remaining,
                explicit_array_segments,
                depth + 1,
                field,
            );
        }
    }
    if let Some(pattern_properties) = object
        .get_mut("patternProperties")
        .and_then(Value::as_object_mut)
    {
        for child in pattern_properties.values_mut() {
            *matched |= annotate_schema_segments(
                child,
                root,
                remaining,
                explicit_array_segments,
                depth + 1,
                field,
            );
        }
    }
}

fn annotate_array_children(
    object: &mut serde_json::Map<String, Value>,
    root: &Value,
    remaining: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    field: &FieldMetadata,
    matched: &mut bool,
) {
    if let Some(children) = object.get_mut("prefixItems").and_then(Value::as_array_mut) {
        for child in children {
            *matched |= annotate_schema_segments(
                child,
                root,
                remaining,
                explicit_array_segments,
                depth + 1,
                field,
            );
        }
    }
    if let Some(children) = object.get_mut("items").and_then(Value::as_array_mut) {
        for child in children {
            *matched |= annotate_schema_segments(
                child,
                root,
                remaining,
                explicit_array_segments,
                depth + 1,
                field,
            );
        }
    }
    if let Some(items) = object.get_mut("items") {
        *matched |= annotate_schema_segments(
            items,
            root,
            remaining,
            explicit_array_segments,
            depth + 1,
            field,
        );
    }
}

fn annotate_dynamic_children(
    object: &mut serde_json::Map<String, Value>,
    root: &Value,
    remaining: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    field: &FieldMetadata,
    matched: &mut bool,
) {
    if let Some(additional) = object
        .get_mut("additionalProperties")
        .filter(|value| value.is_object())
    {
        *matched |= annotate_schema_segments(
            additional,
            root,
            remaining,
            explicit_array_segments,
            depth + 1,
            field,
        );
    }
    let has_legacy_tuple_items = object.get("items").is_some_and(Value::is_array);
    if has_legacy_tuple_items
        && let Some(additional) = object
            .get_mut("additionalItems")
            .filter(|value| value.is_object())
    {
        *matched |= annotate_schema_segments(
            additional,
            root,
            remaining,
            explicit_array_segments,
            depth + 1,
            field,
        );
    }
    if let Some(contains) = object.get_mut("contains").filter(|value| value.is_object()) {
        *matched |= annotate_schema_segments(
            contains,
            root,
            remaining,
            explicit_array_segments,
            depth + 1,
            field,
        );
    }
}

fn annotate_named_child(
    object: &mut serde_json::Map<String, Value>,
    root: &Value,
    segment: &str,
    remaining: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    field: &FieldMetadata,
) -> bool {
    if explicit_array_segments.contains(&depth) {
        return parse_array_index_segment(segment)
            .ok()
            .is_some_and(|index| {
                annotate_tuple_item(
                    object,
                    root,
                    index,
                    remaining,
                    explicit_array_segments,
                    depth,
                    field,
                )
            });
    }

    let mut matched = false;
    if let Some(child) = object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .and_then(|properties| properties.get_mut(segment))
    {
        matched |= annotate_schema_segments(
            child,
            root,
            remaining,
            explicit_array_segments,
            depth + 1,
            field,
        );
    }
    if let Ok(index) = parse_array_index_segment(segment) {
        matched |= annotate_tuple_item(
            object,
            root,
            index,
            remaining,
            explicit_array_segments,
            depth,
            field,
        );
    }
    matched
}

fn annotate_tuple_item(
    object: &mut serde_json::Map<String, Value>,
    root: &Value,
    index: usize,
    remaining: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    field: &FieldMetadata,
) -> bool {
    let mut matched = false;
    if let Some(child) = object
        .get_mut("prefixItems")
        .and_then(Value::as_array_mut)
        .and_then(|items| items.get_mut(index))
    {
        matched |= annotate_schema_segments(
            child,
            root,
            remaining,
            explicit_array_segments,
            depth + 1,
            field,
        );
    }
    if let Some(child) = object
        .get_mut("items")
        .and_then(Value::as_array_mut)
        .and_then(|items| items.get_mut(index))
    {
        matched |= annotate_schema_segments(
            child,
            root,
            remaining,
            explicit_array_segments,
            depth + 1,
            field,
        );
    }
    matched
}

fn inline_schema_ref(node: &mut Value, root: &Value) {
    if let Some(inlined) = inlined_schema_ref(node, root) {
        *node = inlined;
    }
}
