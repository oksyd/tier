use std::collections::{BTreeMap, BTreeSet};

use crate::error::UnknownField;
use crate::loader::SourceTrace;
use crate::loader::path::normalize_external_path;
use crate::report::ConfigReport;

pub(in crate::loader) fn merge_suggestion_paths(
    base: &BTreeMap<String, String>,
    known_paths: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let mut merged = base.clone();
    for path in known_paths {
        merged.entry(path.clone()).or_insert_with(|| path.clone());
    }
    merged
}

pub(in crate::loader) fn unknown_fields_from_paths(
    paths: Vec<String>,
    suggestion_paths: &BTreeMap<String, String>,
    report: &ConfigReport,
) -> Vec<UnknownField> {
    paths
        .into_iter()
        .map(|path| normalize_external_path(&path))
        .filter(|path| !suggestion_paths.contains_key(path))
        .map(|path| {
            let source = find_source_for_unknown_path(report, &path);
            let suggestion = best_path_suggestion(&path, suggestion_paths);
            UnknownField::new(path)
                .with_source(source)
                .with_suggestion(suggestion)
        })
        .collect()
}

pub(in crate::loader) fn find_source_for_unknown_path(
    report: &ConfigReport,
    path: &str,
) -> Option<SourceTrace> {
    let mut current = Some(normalize_external_path(path));
    while let Some(candidate) = current {
        if let Some(source) = report.latest_source_for(&candidate) {
            return Some(source);
        }
        current = candidate
            .rsplit_once('.')
            .map(|(parent, _)| parent.to_owned())
            .filter(|parent| !parent.is_empty());
    }
    None
}

pub(in crate::loader) fn best_path_suggestion(
    path: &str,
    suggestion_paths: &BTreeMap<String, String>,
) -> Option<String> {
    if suggestion_paths.is_empty() {
        return None;
    }

    let normalized = normalize_external_path(path);
    if let Some(suggestion) = best_sibling_path_suggestion(&normalized, suggestion_paths) {
        return Some(suggestion);
    }

    best_global_path_suggestion(&normalized, suggestion_paths)
}

fn best_sibling_path_suggestion(
    normalized: &str,
    suggestion_paths: &BTreeMap<String, String>,
) -> Option<String> {
    let (parent, leaf) = normalized
        .rsplit_once('.')
        .map_or(("", normalized), |(parent, leaf)| (parent, leaf));

    let mut best: Option<(usize, String)> = None;
    for (candidate, canonical) in suggestion_paths {
        let display_candidate = materialize_pattern_for_path(candidate, normalized);
        let display_canonical = materialize_pattern_for_path(canonical, normalized);
        let (candidate_parent, candidate_leaf) = display_candidate
            .rsplit_once('.')
            .map_or(("", display_candidate.as_str()), |(parent, leaf)| {
                (parent, leaf)
            });
        if candidate_parent != parent {
            continue;
        }

        record_better_suggestion(
            &mut best,
            levenshtein(leaf, candidate_leaf),
            display_canonical,
        );
    }

    best.and_then(|(distance, suggestion)| (distance <= 3).then_some(suggestion))
}

fn best_global_path_suggestion(
    normalized: &str,
    suggestion_paths: &BTreeMap<String, String>,
) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for (candidate, canonical) in suggestion_paths {
        let display_candidate = materialize_pattern_for_path(candidate, normalized);
        let display_canonical = materialize_pattern_for_path(canonical, normalized);
        record_better_suggestion(
            &mut best,
            levenshtein(normalized, &display_candidate),
            display_canonical,
        );
    }

    best.and_then(|(distance, suggestion)| {
        let max_len = normalized.len().max(suggestion.len());
        (distance <= (max_len / 3).max(2)).then_some(suggestion)
    })
}

fn record_better_suggestion(
    best: &mut Option<(usize, String)>,
    distance: usize,
    candidate: String,
) {
    match best {
        Some((best_distance, best_candidate)) if distance < *best_distance => {
            *best_distance = distance;
            *best_candidate = candidate;
        }
        None => *best = Some((distance, candidate)),
        _ => {}
    }
}

fn materialize_pattern_for_path(pattern: &str, actual_path: &str) -> String {
    let pattern_segments = pattern
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let actual_segments = actual_path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    pattern_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            if *segment == "*" {
                actual_segments
                    .get(index)
                    .copied()
                    .unwrap_or("<item>")
                    .to_owned()
            } else {
                (*segment).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn levenshtein(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }

    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let cost = usize::from(left_char != *right_char);
            current[right_index + 1] = (current[right_index] + 1)
                .min(previous[right_index + 1] + 1)
                .min(previous[right_index] + cost);
        }
        previous.clone_from_slice(&current);
    }

    previous[right_chars.len()]
}
