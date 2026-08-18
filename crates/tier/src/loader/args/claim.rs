use std::collections::{BTreeMap, BTreeSet};

use crate::ConfigError;
use crate::path::{concrete_paths_overlap, is_array_index_segment, path_child_segments};

pub(super) fn claim_arg_path(
    arg: &str,
    path: &str,
    is_direct_array: bool,
    direct_array_paths: &BTreeSet<String>,
    claimed_paths: &mut BTreeMap<String, String>,
) -> Result<(), ConfigError> {
    for (existing_path, existing_arg) in claimed_paths.iter() {
        if existing_path == path {
            return Err(ConfigError::InvalidArg {
                arg: arg.to_owned(),
                message: format!(
                    "conflicting CLI overrides `{existing_arg}` and `{arg}` both target `{path}`"
                ),
            });
        }

        if concrete_paths_overlap(existing_path, path) {
            if direct_array_overlap_allowed(
                existing_path,
                path,
                is_direct_array,
                direct_array_paths,
            ) {
                continue;
            }
            return Err(ConfigError::InvalidArg {
                arg: arg.to_owned(),
                message: format!(
                    "conflicting CLI overrides `{existing_arg}` and `{arg}` target overlapping configuration paths `{existing_path}` and `{path}`"
                ),
            });
        }
    }

    claimed_paths.insert(path.to_owned(), arg.to_owned());
    Ok(())
}

fn direct_array_overlap_allowed(
    existing_path: &str,
    new_path: &str,
    new_is_direct_array: bool,
    direct_array_paths: &BTreeSet<String>,
) -> bool {
    direct_array_prefix_allows(
        existing_path,
        new_path,
        direct_array_paths.contains(existing_path),
    ) || direct_array_prefix_allows(new_path, existing_path, new_is_direct_array)
}

fn direct_array_prefix_allows(prefix: &str, other: &str, is_direct_array: bool) -> bool {
    if !is_direct_array {
        return false;
    }

    let Some(segments) = path_child_segments(other, prefix) else {
        return false;
    };

    segments
        .first()
        .is_some_and(|segment| is_array_index_segment(segment))
}
