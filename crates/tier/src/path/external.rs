use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExternalPathSegment {
    Field(String),
    Index(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArrayIndexSegment {
    Index(usize),
    NonNumeric,
    Invalid(String),
}

pub(crate) const MAX_ARRAY_INDEX: usize = 1_048_575;

impl ExternalPathSegment {
    pub(crate) fn value(&self) -> &str {
        match self {
            Self::Field(value) | Self::Index(value) => value,
        }
    }
}

pub(crate) fn parse_external_path(path: &str) -> Result<Vec<ExternalPathSegment>, String> {
    if path == "." {
        return Ok(Vec::new());
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();
    let mut after_index = false;
    let mut expecting_segment = true;

    while let Some(ch) = chars.next() {
        if after_index {
            match ch {
                '.' => {
                    if chars.peek().is_none() {
                        return Err("configuration path cannot end with `.`".to_owned());
                    }
                    after_index = false;
                    expecting_segment = true;
                }
                '[' => {
                    let index = parse_array_index(&mut chars)?;
                    segments.push(ExternalPathSegment::Index(index));
                    after_index = true;
                    expecting_segment = false;
                }
                _ => {
                    return Err(
                        "expected `.` or `[` after an array index in configuration path".to_owned(),
                    );
                }
            }
            continue;
        }

        match ch {
            '.' => {
                if current.is_empty() {
                    return Err("empty path segment in configuration path".to_owned());
                }
                segments.push(ExternalPathSegment::Field(std::mem::take(&mut current)));
                expecting_segment = true;
            }
            '[' => {
                if current.is_empty() {
                    return Err("array indices must follow a field name".to_owned());
                }
                segments.push(ExternalPathSegment::Field(std::mem::take(&mut current)));
                let index = parse_array_index(&mut chars)?;
                segments.push(ExternalPathSegment::Index(index));
                after_index = true;
                expecting_segment = false;
            }
            ']' => return Err("unexpected `]` in configuration path".to_owned()),
            _ => {
                current.push(ch);
                expecting_segment = false;
            }
        }
    }

    if expecting_segment && !segments.is_empty() && current.is_empty() && !after_index {
        return Err("configuration path cannot end with `.`".to_owned());
    }

    if !current.is_empty() {
        segments.push(ExternalPathSegment::Field(current));
    }

    Ok(segments)
}

pub(crate) fn render_external_path(segments: &[ExternalPathSegment]) -> String {
    segments
        .iter()
        .map(ExternalPathSegment::value)
        .collect::<Vec<_>>()
        .join(".")
}

pub(crate) fn render_path_with_explicit_array_segments(
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
) -> String {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut rendered = String::new();
    for (index, segment) in segments.into_iter().enumerate() {
        if explicit_array_segments.contains(&index) && !rendered.is_empty() {
            rendered.push('[');
            rendered.push_str(segment);
            rendered.push(']');
            continue;
        }

        if !rendered.is_empty() {
            rendered.push('.');
        }
        rendered.push_str(segment);
    }
    rendered
}

pub(crate) fn normalize_external_path(path: &str) -> Result<String, String> {
    parse_external_path(path).map(|segments| render_external_path(&segments))
}

pub(crate) fn normalize_external_path_with_explicit_arrays(
    path: &str,
) -> Result<(String, BTreeSet<usize>), String> {
    let segments = parse_external_path(path)?;
    let explicit_array_segments = segments
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| {
            matches!(segment, ExternalPathSegment::Index(_)).then_some(index)
        })
        .collect();
    Ok((render_external_path(&segments), explicit_array_segments))
}

fn parse_array_index<I>(chars: &mut std::iter::Peekable<I>) -> Result<String, String>
where
    I: Iterator<Item = char>,
{
    let mut index = String::new();
    let mut closed = false;
    for next in chars.by_ref() {
        if next == ']' {
            closed = true;
            break;
        }
        index.push(next);
    }
    if !closed {
        return Err("unclosed `[` in configuration path".to_owned());
    }
    if index.is_empty() {
        return Err("empty array index in configuration path".to_owned());
    }
    if !index.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("array indices in configuration paths must be numeric".to_owned());
    }
    parse_array_index_segment(&index).map(|value| value.to_string())
}

pub(crate) fn parse_array_index_segment(segment: &str) -> Result<usize, String> {
    match classify_array_index_segment(segment) {
        ArrayIndexSegment::Index(index) => Ok(index),
        ArrayIndexSegment::NonNumeric => {
            Err("array indices in configuration paths must be numeric".to_owned())
        }
        ArrayIndexSegment::Invalid(message) => Err(message),
    }
}

pub(crate) fn classify_array_index_segment(segment: &str) -> ArrayIndexSegment {
    if segment.is_empty() || !segment.chars().all(|ch| ch.is_ascii_digit()) {
        return ArrayIndexSegment::NonNumeric;
    }
    let index = segment
        .parse::<usize>()
        .map_err(|_| "array indices in configuration paths must fit in usize".to_owned());
    let index = match index {
        Ok(index) => index,
        Err(message) => return ArrayIndexSegment::Invalid(message),
    };
    if let Err(message) = checked_array_len_for_index(index) {
        return ArrayIndexSegment::Invalid(message);
    }
    ArrayIndexSegment::Index(index)
}

pub(crate) fn is_array_index_segment(segment: &str) -> bool {
    matches!(
        classify_array_index_segment(segment),
        ArrayIndexSegment::Index(_)
    )
}

pub(crate) fn checked_array_len_for_index(index: usize) -> Result<usize, String> {
    if index > MAX_ARRAY_INDEX {
        return Err(format!(
            "array indices in configuration paths must be <= {MAX_ARRAY_INDEX}"
        ));
    }
    index
        .checked_add(1)
        .ok_or_else(|| "array index is too large".to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::render_path_with_explicit_array_segments;

    #[test]
    fn renders_explicit_array_segments_as_external_bracket_paths() {
        assert_eq!(
            render_path_with_explicit_array_segments(
                "matrix.0.1.password",
                &BTreeSet::from([1, 2]),
            ),
            "matrix[0][1].password"
        );
    }

    #[test]
    fn renders_dot_paths_without_explicit_array_segments_unchanged() {
        assert_eq!(
            render_path_with_explicit_array_segments("value.0.password", &BTreeSet::new()),
            "value.0.password"
        );
    }
}
