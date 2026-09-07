use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::ConfigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchTargetKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub(super) struct WatchTarget {
    path: PathBuf,
    kind: WatchTargetKind,
    watch_root: PathBuf,
    recursive: bool,
}

impl WatchTarget {
    pub(super) fn matches_event_path(&self, path: &Path) -> bool {
        match self.kind {
            WatchTargetKind::File => path == self.path,
            WatchTargetKind::Directory => path == self.path || path.starts_with(&self.path),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct WatchRegistration {
    pub(super) root: PathBuf,
    pub(super) recursive: bool,
}

pub(super) fn prepare_watch_targets(paths: Vec<PathBuf>) -> Result<Vec<WatchTarget>, ConfigError> {
    let mut targets = Vec::new();
    for path in paths {
        expand_watch_path(absolutize_path(&path)?, &mut targets, &mut BTreeSet::new())?;
    }
    Ok(targets)
}

fn expand_watch_path(
    path: PathBuf,
    targets: &mut Vec<WatchTarget>,
    seen: &mut BTreeSet<PathBuf>,
) -> Result<(), ConfigError> {
    if !seen.insert(path.clone()) || seen.len() > 64 {
        return Ok(());
    }
    let ancestors: Vec<_> = path.ancestors().collect();
    for ancestor in ancestors.into_iter().rev() {
        if std::fs::symlink_metadata(ancestor).is_ok_and(|m| m.file_type().is_symlink()) {
            // Watch the link entry through its parent, including directory links.
            let parent = ancestor.parent().unwrap_or(ancestor);
            let (watch_root, recursive) = watch_root_for_parent(parent)?;
            targets.push(WatchTarget {
                path: ancestor.to_path_buf(),
                kind: WatchTargetKind::File,
                watch_root,
                recursive,
            });
            if let Ok(destination) = std::fs::read_link(ancestor) {
                let destination = if destination.is_absolute() {
                    destination
                } else {
                    parent.join(destination)
                };
                if let Ok(suffix) = path.strip_prefix(ancestor) {
                    expand_watch_path(destination.join(suffix), targets, seen)?;
                }
            }
            return Ok(());
        }
    }
    targets.push(watch_target_for_path(path)?);
    Ok(())
}

fn watch_target_for_path(path: PathBuf) -> Result<WatchTarget, ConfigError> {
    if path.exists() && path.is_dir() {
        return Ok(WatchTarget {
            watch_root: path.clone(),
            path,
            kind: WatchTargetKind::Directory,
            recursive: true,
        });
    }

    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| path.clone());
    let (watch_root, recursive) = watch_root_for_parent(&parent)?;

    Ok(WatchTarget {
        path,
        kind: WatchTargetKind::File,
        watch_root,
        recursive,
    })
}

fn watch_root_for_parent(parent: &Path) -> Result<(PathBuf, bool), ConfigError> {
    if parent.exists() {
        Ok((parent.to_path_buf(), false))
    } else if let Some(root) = nearest_existing_ancestor(parent) {
        Ok((root, true))
    } else {
        Ok((std::env::current_dir().map_err(map_watch_io_error)?, true))
    }
}

pub(super) fn collect_watch_registrations(targets: &[WatchTarget]) -> Vec<WatchRegistration> {
    let mut registrations = BTreeMap::<PathBuf, bool>::new();
    for target in targets {
        registrations
            .entry(target.watch_root.clone())
            .and_modify(|recursive| *recursive |= target.recursive)
            .or_insert(target.recursive);
    }

    registrations
        .into_iter()
        .map(|(root, recursive)| WatchRegistration { root, recursive })
        .collect()
}

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

fn absolutize_path(path: &Path) -> Result<PathBuf, ConfigError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .map_err(map_watch_io_error)?
            .join(path))
    }
}

pub(super) fn absolutize_event_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        std::env::current_dir().ok().map(|cwd| cwd.join(path))
    }
}

fn map_watch_io_error(error: std::io::Error) -> ConfigError {
    ConfigError::Watch {
        message: error.to_string(),
    }
}
