use std::collections::BTreeMap;

use super::parse_array_index_segment;

pub(crate) fn normalize_path(path: &str) -> String {
    path.trim_matches('.').to_owned()
}

pub(crate) fn join_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_owned()
    } else {
        format!("{parent}.{child}")
    }
}

pub(crate) fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    let actual_segments = path_segments(path);
    let pattern_segments = path_segments(pattern);
    actual_segments.len() == pattern_segments.len()
        && actual_segments
            .iter()
            .zip(pattern_segments.iter())
            .all(|(actual, expected)| *expected == "*" || actual == expected)
}

pub(crate) fn path_starts_with_pattern(path: &str, pattern: &str) -> bool {
    let actual_segments = path_segments(path);
    let pattern_segments = path_segments(pattern);
    actual_segments.len() >= pattern_segments.len()
        && actual_segments
            .iter()
            .zip(pattern_segments.iter())
            .all(|(actual, expected)| *expected == "*" || actual == expected)
}

pub(crate) fn path_overlaps_pattern(path: &str, pattern: &str) -> bool {
    let actual_segments = path_segments(path);
    let pattern_segments = path_segments(pattern);
    let shared = actual_segments.len().min(pattern_segments.len());
    actual_segments
        .iter()
        .take(shared)
        .zip(pattern_segments.iter().take(shared))
        .all(|(actual, expected)| *expected == "*" || *actual == "*" || actual == expected)
}

pub(crate) fn path_is_at_or_below(path: &str, parent: &str) -> bool {
    path_child_segments(path, parent).is_some()
}

pub(crate) fn path_child_segments<'a>(path: &'a str, parent: &str) -> Option<Vec<&'a str>> {
    let child_segments = path_segments(path);
    let parent_segments = path_segments(parent);
    if child_segments.len() < parent_segments.len()
        || !child_segments
            .iter()
            .zip(parent_segments.iter())
            .all(|(actual, expected)| actual == expected)
    {
        return None;
    }

    Some(child_segments[parent_segments.len()..].to_vec())
}

pub(crate) fn direct_child_array_index(container_path: &str, entry_path: &str) -> Option<usize> {
    path_child_segments(entry_path, container_path)?
        .first()
        .and_then(|segment| parse_array_index_segment(segment).ok())
}

pub(crate) fn concrete_paths_overlap(left: &str, right: &str) -> bool {
    path_is_at_or_below(left, right) || path_is_at_or_below(right, left)
}

pub(crate) fn canonicalize_path_with_aliases(
    path: &str,
    aliases: &BTreeMap<String, String>,
) -> String {
    let normalized = normalize_path(path);
    if normalized.is_empty() || aliases.is_empty() {
        return normalized;
    }

    best_alias_rewrite(&normalized, aliases).map_or(normalized, |rewrite| rewrite.path)
}

struct AliasRewrite {
    alias_len: usize,
    specificity: usize,
    path: String,
}

fn best_alias_rewrite(
    normalized: &str,
    aliases: &BTreeMap<String, String>,
) -> Option<AliasRewrite> {
    let path_segments = normalized.split('.').collect::<Vec<_>>();
    let mut best = None::<AliasRewrite>;

    for (alias, canonical) in aliases {
        let alias_segments = alias.split('.').collect::<Vec<_>>();
        if alias_segments.len() > path_segments.len() {
            continue;
        }

        let matched = alias_segments
            .iter()
            .zip(path_segments.iter())
            .all(|(expected, actual)| *expected == "*" || expected == actual);
        if !matched {
            continue;
        }

        let specificity = alias_segments
            .iter()
            .filter(|segment| **segment != "*")
            .count();
        let rewrite = AliasRewrite {
            alias_len: alias_segments.len(),
            specificity,
            path: rewrite_alias_path(&path_segments, &alias_segments, canonical),
        };

        match &mut best {
            Some(best)
                if rewrite.alias_len > best.alias_len
                    || (rewrite.alias_len == best.alias_len
                        && rewrite.specificity > best.specificity) =>
            {
                *best = rewrite;
            }
            None => best = Some(rewrite),
            _ => {}
        }
    }

    best
}

fn rewrite_alias_path(path_segments: &[&str], alias_segments: &[&str], canonical: &str) -> String {
    let canonical_segments = canonical
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut rewritten = canonical_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            if *segment == "*" && alias_segments.get(index) == Some(&"*") {
                path_segments[index].to_owned()
            } else {
                (*segment).to_owned()
            }
        })
        .collect::<Vec<_>>();
    rewritten.extend(
        path_segments[alias_segments.len()..]
            .iter()
            .map(|segment| (*segment).to_owned()),
    );
    normalize_path(&rewritten.join("."))
}

pub(crate) fn path_segments(path: &str) -> Vec<&str> {
    path.split('.')
        .filter(|segment| !segment.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        concrete_paths_overlap, direct_child_array_index, path_child_segments, path_is_at_or_below,
    };

    #[test]
    fn concrete_path_prefix_checks_are_segment_aware() {
        assert!(path_is_at_or_below("db.token", "db"));
        assert!(path_is_at_or_below("db", "db"));
        assert!(path_is_at_or_below("db", ""));
        assert!(!path_is_at_or_below("database.token", "db"));
        assert!(!path_is_at_or_below("db", "db.token"));
    }

    #[test]
    fn concrete_path_overlap_checks_are_segment_aware() {
        assert!(concrete_paths_overlap("db", "db.token"));
        assert!(concrete_paths_overlap("db.token", "db"));
        assert!(concrete_paths_overlap("db.token", "db.token"));
        assert!(!concrete_paths_overlap("db", "database"));
        assert!(!concrete_paths_overlap("db.token", "db_token"));
    }

    #[test]
    fn child_path_segments_return_descendant_tail() {
        assert_eq!(
            path_child_segments("users.0.name", "users"),
            Some(vec!["0", "name"])
        );
        assert_eq!(
            path_child_segments("users.0.name", ""),
            Some(vec!["users", "0", "name"])
        );
        assert_eq!(path_child_segments("users", "users"), Some(Vec::new()));
        assert_eq!(path_child_segments("users_profile.0", "users"), None);
    }

    #[test]
    fn direct_child_array_index_is_segment_aware() {
        assert_eq!(direct_child_array_index("users", "users.0.name"), Some(0));
        assert_eq!(direct_child_array_index("", "0.name"), Some(0));
        assert_eq!(direct_child_array_index("users", "users_profile.0"), None);
        assert_eq!(direct_child_array_index("users.0", "users.0.name"), None);
    }
}
