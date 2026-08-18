use std::collections::BTreeSet;

use serde_json::Value;

use crate::ConfigMetadata;
use crate::path::path_segments;

use super::comments::{
    render_toml_array_table_item_comments, render_toml_comments,
    render_toml_inline_array_item_comments,
};
use super::path::{
    join_metadata_path, resolve_toml_array_item_metadata_path,
    resolve_toml_object_child_metadata_path,
};
use super::value::{
    is_nested_toml_value, start_toml_section, toml_inline_value, toml_key, toml_table_name,
};

pub(super) fn render_example_toml(value: &Value, metadata: &ConfigMetadata) -> String {
    let mut output = String::new();
    render_toml_root_comments(metadata, &mut output);
    let explicit_array_segments = BTreeSet::new();
    match value {
        Value::Object(root) => render_toml_table(
            "",
            "",
            &explicit_array_segments,
            root,
            metadata,
            &mut output,
        ),
        other => render_toml_root_value(other, &explicit_array_segments, metadata, &mut output),
    }

    if !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

fn render_toml_root_value(
    value: &Value,
    explicit_array_segments: &BTreeSet<usize>,
    metadata: &ConfigMetadata,
    output: &mut String,
) {
    if matches!(value, Value::Array(_)) {
        render_toml_inline_array_item_comments("", explicit_array_segments, metadata, output);
    }

    output.push_str(
        "# root values are rendered as comments because TOML documents require a table at the top level\n",
    );
    output.push_str("# ");
    output.push_str(&toml_inline_value(value));
    output.push('\n');
}

fn render_toml_root_comments(metadata: &ConfigMetadata, output: &mut String) {
    if metadata.checks().is_empty() {
        return;
    }

    for check in metadata.checks() {
        output.push_str("# validate: ");
        output.push_str(&check.to_string());
        output.push('\n');
    }

    output.push('\n');
}

fn render_toml_table(
    display_path: &str,
    metadata_path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    table: &serde_json::Map<String, Value>,
    metadata: &ConfigMetadata,
    output: &mut String,
) {
    let mut nested = Vec::new();

    for (key, value) in table {
        if value.is_null() {
            continue;
        }

        if is_nested_toml_value(value) {
            nested.push((key.as_str(), value));
            continue;
        }

        let metadata_child_path =
            resolve_toml_object_child_metadata_path(metadata_path, key, metadata);
        let metadata_child_explicit_array_segments = explicit_array_segments.clone();
        render_toml_comments(
            &metadata_child_path,
            &metadata_child_explicit_array_segments,
            metadata,
            output,
        );
        if matches!(value, Value::Array(_)) {
            render_toml_inline_array_item_comments(
                &metadata_child_path,
                &metadata_child_explicit_array_segments,
                metadata,
                output,
            );
        }
        output.push_str(&toml_key(key));
        output.push_str(" = ");
        output.push_str(&toml_inline_value(value));
        output.push('\n');
    }

    let nested_count = nested.len();
    for (index, (key, value)) in nested.into_iter().enumerate() {
        let display_child_path = join_metadata_path(display_path, key);
        let metadata_child_path =
            resolve_toml_object_child_metadata_path(metadata_path, key, metadata);
        let metadata_child_explicit_array_segments = explicit_array_segments.clone();
        match value {
            Value::Object(child) => {
                start_toml_section(output);
                render_toml_comments(
                    &metadata_child_path,
                    &metadata_child_explicit_array_segments,
                    metadata,
                    output,
                );
                output.push('[');
                output.push_str(&toml_table_name(&display_child_path));
                output.push_str("]\n");
                render_toml_table(
                    &display_child_path,
                    &metadata_child_path,
                    &metadata_child_explicit_array_segments,
                    child,
                    metadata,
                    output,
                );
            }
            Value::Array(items) => {
                for (item_index, item) in items.iter().enumerate() {
                    let Some(child) = item.as_object() else {
                        continue;
                    };
                    let item_metadata_path = resolve_toml_array_item_metadata_path(
                        &metadata_child_path,
                        item_index,
                        metadata,
                    );
                    let item_explicit_array_segments = array_item_explicit_array_segments(
                        &metadata_child_explicit_array_segments,
                        &item_metadata_path,
                    );
                    start_toml_section(output);
                    if item_index == 0 {
                        render_toml_comments(
                            &metadata_child_path,
                            &metadata_child_explicit_array_segments,
                            metadata,
                            output,
                        );
                    }
                    render_toml_array_table_item_comments(
                        &item_metadata_path,
                        &item_explicit_array_segments,
                        metadata,
                        output,
                    );
                    output.push_str("[[");
                    output.push_str(&toml_table_name(&display_child_path));
                    output.push_str("]]\n");
                    render_toml_table(
                        &display_child_path,
                        &item_metadata_path,
                        &item_explicit_array_segments,
                        child,
                        metadata,
                        output,
                    );
                }
            }
            _ => {}
        }

        if index + 1 < nested_count && !output.ends_with("\n\n") {
            output.push('\n');
        }
    }
}

fn array_item_explicit_array_segments(
    parent_explicit_array_segments: &BTreeSet<usize>,
    item_path: &str,
) -> BTreeSet<usize> {
    let mut explicit_array_segments = parent_explicit_array_segments.clone();
    if !item_path.is_empty() {
        explicit_array_segments.insert(path_segments(item_path).len() - 1);
    }
    explicit_array_segments
}
