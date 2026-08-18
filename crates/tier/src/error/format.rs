use std::path::PathBuf;

use crate::loader::{SourceKind, SourceTrace};

use super::{LineColumn, UnknownField};

pub(super) fn format_location(location: Option<LineColumn>) -> String {
    match location {
        Some(location) => format!(" ({location})"),
        None => String::new(),
    }
}

pub(super) fn format_missing_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| format!("- {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn format_unknown_fields(fields: &[UnknownField]) -> String {
    fields
        .iter()
        .map(|field| format!("- {field}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn format_path_location(path: &str) -> String {
    if path.is_empty() {
        "the configuration root".to_owned()
    } else {
        format!("`{path}`")
    }
}

pub(super) fn format_source_policy(allowed: &[SourceKind], denied: &[SourceKind]) -> String {
    match (allowed.is_empty(), denied.is_empty()) {
        (false, true) => format!("allowed sources: {}", format_source_kind_list(allowed)),
        (true, false) => format!("denied sources: {}", format_source_kind_list(denied)),
        (false, false) => format!(
            "allowed sources: {}; denied sources: {}",
            format_source_kind_list(allowed),
            format_source_kind_list(denied)
        ),
        (true, true) => "no source policy matched".to_owned(),
    }
}

pub(super) fn deserialize_source_suffix(provenance: &Option<SourceTrace>) -> String {
    provenance
        .as_ref()
        .map_or_else(String::new, |source| format!(" from {source}"))
}

fn format_source_kind_list(kinds: &[SourceKind]) -> String {
    kinds
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
