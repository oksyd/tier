use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::path::{
    ArrayIndexSegment, checked_array_len_for_index, classify_array_index_segment,
    parse_array_index_segment,
};

#[derive(Clone, Copy)]
enum NextContainer {
    Object,
    Array,
}

impl NextContainer {
    fn empty_value(self) -> Value {
        match self {
            Self::Object => Value::Object(Map::new()),
            Self::Array => Value::Array(Vec::new()),
        }
    }
}

pub(crate) fn insert_path_with_shape_and_explicit_arrays(
    root: &mut Value,
    shape: Option<&Value>,
    segments: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    value: Value,
) -> Result<(), String> {
    if segments.is_empty() {
        return Err("configuration path cannot be empty".to_owned());
    }

    insert_path_recursive(root, shape, segments, explicit_array_segments, 0, value)
}

fn insert_path_recursive(
    current: &mut Value,
    shape: Option<&Value>,
    segments: &[&str],
    explicit_array_segments: &BTreeSet<usize>,
    depth: usize,
    value: Value,
) -> Result<(), String> {
    let segment = segments[0];
    if segment.is_empty() {
        return Err("configuration path contains an empty segment".to_owned());
    }

    let is_last = segments.len() == 1;
    match current {
        Value::Object(map) => {
            if is_last {
                map.insert(segment.to_owned(), value);
                return Ok(());
            }

            let shape_child = object_shape_child(shape, segment);
            let next_is_explicit_array = explicit_array_segments.contains(&(depth + 1));
            let next_container = match map.get(segment) {
                Some(child) => {
                    existing_child_container(child, segments[1], segment, next_is_explicit_array)?
                }
                None if shape_child.is_some() => shape_child_container(
                    shape_child,
                    segments[1],
                    segment,
                    next_is_explicit_array,
                )?,
                None => infer_next_container(segments[1], next_is_explicit_array)?,
            };
            let child = map
                .entry(segment.to_owned())
                .or_insert_with(|| next_container.empty_value());
            ensure_container(child, next_container, segment)?;

            insert_path_recursive(
                child,
                shape_child,
                &segments[1..],
                explicit_array_segments,
                depth + 1,
                value,
            )
        }
        Value::Array(values) => {
            let index = parse_array_index_segment(segment).map_err(|message| {
                format!("path segment {segment} must be an array index at this position: {message}")
            })?;

            if is_last {
                if values.len() <= index {
                    values.resize(checked_array_len_for_index(index)?, Value::Null);
                }
                values[index] = value;
                return Ok(());
            }

            let shape_child = array_shape_child(shape, index);
            let next_is_explicit_array = explicit_array_segments.contains(&(depth + 1));
            let next_container = values
                .get(index)
                .filter(|child| !child.is_null())
                .map(|child| {
                    existing_child_container(child, segments[1], segment, next_is_explicit_array)
                })
                .or_else(|| {
                    shape_child.map(|child| {
                        shape_child_container(
                            Some(child),
                            segments[1],
                            segment,
                            next_is_explicit_array,
                        )
                    })
                })
                .unwrap_or_else(|| infer_next_container(segments[1], next_is_explicit_array))?;
            if values.len() <= index {
                values.resize_with(checked_array_len_for_index(index)?, || {
                    next_container.empty_value()
                });
            }

            let child = &mut values[index];
            if child.is_null() {
                *child = next_container.empty_value();
            }
            ensure_container(child, next_container, segment)?;

            insert_path_recursive(
                child,
                shape_child,
                &segments[1..],
                explicit_array_segments,
                depth + 1,
                value,
            )
        }
        _ => Err(format!(
            "path segment {segment} conflicts with an existing non-container value"
        )),
    }
}

fn object_shape_child<'a>(shape: Option<&'a Value>, segment: &str) -> Option<&'a Value> {
    match shape {
        Some(Value::Object(map)) => map.get(segment),
        _ => None,
    }
}

fn array_shape_child(shape: Option<&Value>, index: usize) -> Option<&Value> {
    match shape {
        Some(Value::Array(values)) => values.get(index),
        _ => None,
    }
}

fn existing_child_container(
    child: &Value,
    next_segment: &str,
    segment: &str,
    next_is_explicit_array: bool,
) -> Result<NextContainer, String> {
    match child {
        Value::Object(_) if next_is_explicit_array => Err(format!(
            "path segment {next_segment} uses array syntax after existing object segment {segment}"
        )),
        Value::Object(_) => Ok(NextContainer::Object),
        Value::Array(_) => {
            infer_next_container(next_segment, next_is_explicit_array).and_then(|container| {
                match container {
                    NextContainer::Array => Ok(NextContainer::Array),
                    NextContainer::Object => Err(format!(
                        "path segment {next_segment} must be an array index after {segment}"
                    )),
                }
            })
        }
        _ => Err(format!(
            "path segment {segment} conflicts with an existing non-container value"
        )),
    }
}

fn shape_child_container(
    shape_child: Option<&Value>,
    next_segment: &str,
    segment: &str,
    next_is_explicit_array: bool,
) -> Result<NextContainer, String> {
    match shape_child {
        Some(Value::Object(map)) if map.is_empty() && next_is_explicit_array => {
            Ok(NextContainer::Array)
        }
        Some(Value::Object(_)) if next_is_explicit_array => Err(format!(
            "path segment {next_segment} uses array syntax after object segment {segment}"
        )),
        Some(Value::Object(_)) => Ok(NextContainer::Object),
        Some(Value::Array(_)) => infer_next_container(next_segment, next_is_explicit_array)
            .and_then(|container| match container {
                NextContainer::Array => Ok(NextContainer::Array),
                NextContainer::Object => Err(format!(
                    "path segment {next_segment} must be an array index after {segment}"
                )),
            }),
        Some(_) => Err(format!(
            "path segment {segment} conflicts with an existing non-container value"
        )),
        None => infer_next_container(next_segment, next_is_explicit_array),
    }
}

fn infer_next_container(segment: &str, is_explicit_array: bool) -> Result<NextContainer, String> {
    if is_explicit_array {
        parse_array_index_segment(segment).map_err(|message| {
            format!(
                "path segment {segment} must be a valid array index at this position: {message}"
            )
        })?;
        return Ok(NextContainer::Array);
    }

    match classify_array_index_segment(segment) {
        ArrayIndexSegment::Index(_) => Ok(NextContainer::Array),
        ArrayIndexSegment::NonNumeric => Ok(NextContainer::Object),
        ArrayIndexSegment::Invalid(message) => Err(format!(
            "path segment {segment} must be a valid array index at this position: {message}"
        )),
    }
}

fn ensure_container(child: &Value, expected: NextContainer, segment: &str) -> Result<(), String> {
    match (child, expected) {
        (Value::Object(_), NextContainer::Object) | (Value::Array(_), NextContainer::Array) => {
            Ok(())
        }
        _ => Err(format!(
            "path segment {segment} conflicts with an existing non-container value"
        )),
    }
}
