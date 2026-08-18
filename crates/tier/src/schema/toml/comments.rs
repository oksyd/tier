use std::collections::{BTreeMap, BTreeSet};

use crate::path::{parse_array_index_segment, path_child_segments, path_segments};
use crate::{ConfigMetadata, FieldMetadata, MergeStrategy};

pub(super) fn render_toml_comments(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &ConfigMetadata,
    output: &mut String,
) {
    for field in toml_comment_fields(path, explicit_array_segments, metadata) {
        for line in toml_comment_doc_lines(field) {
            output.push_str("# ");
            output.push_str(&line);
            output.push('\n');
        }
    }
    for line in toml_effective_comment_lines(path, explicit_array_segments, metadata) {
        output.push_str("# ");
        output.push_str(&line);
        output.push('\n');
    }
}

pub(super) fn render_toml_array_table_item_comments(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &ConfigMetadata,
    output: &mut String,
) {
    for field in toml_comment_fields(path, explicit_array_segments, metadata) {
        for line in toml_comment_doc_lines(field) {
            output.push_str("# ");
            output.push_str(&line);
            output.push('\n');
        }
    }
    for line in toml_effective_comment_lines(path, explicit_array_segments, metadata) {
        output.push_str("# ");
        output.push_str(&line);
        output.push('\n');
    }
}

pub(super) fn render_toml_inline_array_item_comments(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &ConfigMetadata,
    output: &mut String,
) {
    let mut wildcard = None::<&FieldMetadata>;
    let mut indexed = BTreeMap::<Vec<TomlArrayCommentSegment>, &FieldMetadata>::new();
    for field in metadata.fields() {
        if !field_parent_intent_matches(field, path, explicit_array_segments) {
            continue;
        }
        let Some(segments) = path_child_segments(&field.path, path) else {
            continue;
        };
        if segments.as_slice() == ["*"] {
            wildcard = Some(field);
            continue;
        }
        if segments.is_empty() {
            continue;
        }
        if let Some(segments) = parse_toml_comment_segments(&segments) {
            indexed.insert(segments, field);
        }
    }

    if let Some(field) = wildcard {
        for line in toml_comment_lines(field) {
            output.push_str("# [*] ");
            output.push_str(&line);
            output.push('\n');
        }
    }

    for (segments, field) in indexed {
        let marker = segments
            .iter()
            .map(|segment| match segment {
                TomlArrayCommentSegment::Wildcard => "[*]".to_owned(),
                TomlArrayCommentSegment::Index(index) => format!("[{index}]"),
            })
            .collect::<String>();
        let mut lines = toml_comment_doc_lines(field);
        let effective = metadata
            .effective_field_for_path_with_intent(&field.path, &field.path_explicit_array_segments)
            .unwrap_or_else(|| field.clone());
        lines.extend(toml_non_doc_comment_lines(&effective));
        for line in lines {
            output.push_str("# ");
            output.push_str(&marker);
            output.push(' ');
            output.push_str(&line);
            output.push('\n');
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TomlArrayCommentSegment {
    Wildcard,
    Index(usize),
}

fn parse_toml_comment_segments(segments: &[&str]) -> Option<Vec<TomlArrayCommentSegment>> {
    segments
        .iter()
        .map(|segment| {
            if *segment == "*" {
                Some(TomlArrayCommentSegment::Wildcard)
            } else {
                parse_array_index_segment(segment)
                    .ok()
                    .map(TomlArrayCommentSegment::Index)
            }
        })
        .collect()
}

fn toml_comment_sort_key(path: &str) -> (usize, usize) {
    let segments = path.split('.').filter(|segment| !segment.is_empty());
    let total = segments.clone().count();
    let specificity = segments.filter(|segment| *segment != "*").count();
    (specificity, total)
}

fn toml_comment_fields<'a>(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &'a ConfigMetadata,
) -> Vec<&'a FieldMetadata> {
    let mut fields = metadata.matching_fields_for_path_with_intent(path, explicit_array_segments);
    fields.sort_by(|left, right| {
        toml_comment_sort_key(&left.path)
            .cmp(&toml_comment_sort_key(&right.path))
            .then_with(|| left.path.cmp(&right.path))
    });
    fields
}

fn toml_comment_lines(field: &FieldMetadata) -> Vec<String> {
    let mut lines = Vec::new();
    lines.extend(toml_comment_doc_lines(field));
    lines.extend(toml_non_doc_comment_lines(field));
    lines
}

fn toml_comment_doc_lines(field: &FieldMetadata) -> Vec<String> {
    field
        .doc
        .as_ref()
        .map(|doc| {
            doc.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn toml_non_doc_comment_lines(field: &FieldMetadata) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(env) = &field.env {
        lines.push(format!("env: {env}"));
    }
    if !field.aliases.is_empty() {
        lines.push(format!("aliases: {}", field.aliases.join(", ")));
    }
    if field.has_default {
        lines.push("default: provided by serde".to_owned());
    }
    if field.merge_explicit || field.merge != MergeStrategy::Merge {
        lines.push(format!("merge: {}", field.merge));
    }
    if !field.validations.is_empty() {
        lines.push(format!(
            "validate: {}",
            field
                .validations
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if field.secret {
        lines.push("secret: true".to_owned());
    }
    if let Some(note) = &field.deprecated {
        lines.push(format!("deprecated: {note}"));
    }
    lines
}

fn toml_effective_comment_lines(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &ConfigMetadata,
) -> Vec<String> {
    metadata
        .effective_field_for_path_with_intent(path, explicit_array_segments)
        .map(|field| toml_non_doc_comment_lines(&field))
        .unwrap_or_default()
}

fn field_parent_intent_matches(
    field: &FieldMetadata,
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
) -> bool {
    let parent_len = path_segments(path).len();
    if parent_len == 0 {
        return true;
    }

    let query_segments = path_segments(path);
    let field_segments = path_segments(&field.path);
    field
        .path_explicit_array_segments
        .difference(explicit_array_segments)
        .filter(|index| **index < parent_len)
        .all(|index| {
            query_segments.get(*index) == Some(&"*") || field_segments.get(*index) == Some(&"*")
        })
}
