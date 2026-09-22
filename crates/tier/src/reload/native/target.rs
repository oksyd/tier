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
        // A replaced ancestor invalidates filesystem subscriptions even when
        // the requested file name stays the same.
        if self.path.starts_with(path) {
            return true;
        }
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
    pub(super) required: bool,
}

pub(super) fn prepare_watch_targets(paths: Vec<PathBuf>) -> Result<Vec<WatchTarget>, ConfigError> {
    let mut targets = Vec::new();
    let mut directories = BTreeSet::new();
    for path in paths {
        expand_watch_path(
            absolutize_path(&path)?,
            &mut targets,
            &mut BTreeSet::new(),
            &mut directories,
        )?;
    }
    Ok(targets)
}

fn expand_watch_path(
    path: PathBuf,
    targets: &mut Vec<WatchTarget>,
    seen: &mut BTreeSet<PathBuf>,
    directories: &mut BTreeSet<PathBuf>,
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
                path: ancestor.file_name().map_or_else(
                    || ancestor.to_path_buf(),
                    |name| normalize_resolved_path(parent.to_path_buf()).join(name),
                ),
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
                    expand_watch_path(destination.join(suffix), targets, seen, directories)?;
                }
            }
            return Ok(());
        }
    }
    let target = watch_target_for_path(path.clone())?;
    let is_dir = target.kind == WatchTargetKind::Directory;
    targets.push(target);
    if is_dir {
        expand_directory_links(&path, targets, directories)?;
    }
    Ok(())
}

fn expand_directory_links(
    path: &Path,
    targets: &mut Vec<WatchTarget>,
    directories: &mut BTreeSet<PathBuf>,
) -> Result<(), ConfigError> {
    let Ok(canonical) = std::fs::canonicalize(path) else {
        return Ok(());
    };
    if !directories.insert(canonical) {
        return Ok(());
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            expand_watch_path(entry.path(), targets, &mut BTreeSet::new(), directories)?;
        } else if kind.is_dir() {
            expand_directory_links(&entry.path(), targets, directories)?;
        }
    }
    Ok(())
}

fn watch_target_for_path(path: PathBuf) -> Result<WatchTarget, ConfigError> {
    let path = normalize_resolved_path(path);
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
        Ok((normalize_resolved_path(parent.to_path_buf()), false))
    } else if let Some(root) = nearest_existing_ancestor(parent) {
        Ok((normalize_resolved_path(root), true))
    } else {
        Ok((std::env::current_dir().map_err(map_watch_io_error)?, true))
    }
}

fn normalize_resolved_path(path: PathBuf) -> PathBuf {
    // Expand symlinks before reaching this helper, so resolving `..` preserves
    // filesystem semantics. A missing target still has an existing ancestor.
    for ancestor in path.ancestors() {
        if let Ok(canonical) = std::fs::canonicalize(ancestor)
            && let Ok(suffix) = path.strip_prefix(ancestor)
        {
            return canonical.join(suffix);
        }
    }
    path
}

pub(super) fn collect_watch_registrations(targets: &[WatchTarget]) -> Vec<WatchRegistration> {
    let mut registrations = BTreeMap::<PathBuf, (bool, bool)>::new();
    for target in targets {
        // Watch ancestor entries without recursion so directory replacements
        // can rebuild the subscriptions below them.
        for ancestor in target.watch_root.ancestors().skip(1) {
            registrations
                .entry(ancestor.to_path_buf())
                .or_insert((false, false));
        }
        registrations
            .entry(target.watch_root.clone())
            .and_modify(|(recursive, required)| {
                *recursive |= target.recursive;
                *required = true;
            })
            .or_insert((target.recursive, true));
    }

    let mut registrations: Vec<_> = registrations
        .into_iter()
        .map(|(root, (recursive, required))| WatchRegistration {
            root,
            recursive,
            required,
        })
        .collect();
    // Install required roots first; optional distant ancestors must not consume
    // a limited watch budget before the requested paths are registered.
    registrations.sort_by_key(|registration| {
        (
            !registration.required,
            std::cmp::Reverse(registration.root.components().count()),
        )
    });
    registrations
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use notify::{RecursiveMode, Watcher};

    use super::{WatchTarget, WatchTargetKind, collect_watch_registrations};

    #[derive(Default)]
    struct RestrictedWatcher {
        denied: BTreeSet<PathBuf>,
        watched: BTreeSet<PathBuf>,
    }

    impl Watcher for RestrictedWatcher {
        fn new<F: notify::EventHandler>(_: F, _: notify::Config) -> notify::Result<Self> {
            Ok(Self::default())
        }

        fn watch(&mut self, path: &Path, _: RecursiveMode) -> notify::Result<()> {
            if self.denied.contains(path) {
                return Err(notify::Error::io(std::io::Error::from(
                    std::io::ErrorKind::PermissionDenied,
                )));
            }
            self.watched.insert(path.to_path_buf());
            Ok(())
        }

        fn unwatch(&mut self, path: &Path) -> notify::Result<()> {
            self.watched.remove(path);
            Ok(())
        }

        fn kind() -> notify::WatcherKind {
            notify::WatcherKind::NullWatcher
        }
    }

    fn file_target(root: &Path) -> WatchTarget {
        WatchTarget {
            path: root.join("config"),
            kind: WatchTargetKind::File,
            watch_root: root.to_path_buf(),
            recursive: false,
        }
    }

    #[test]
    fn unreadable_extra_ancestors_do_not_prevent_required_or_nearby_watches() -> notify::Result<()>
    {
        let base = std::env::temp_dir();
        let near = base.join("tier-registration-test");
        let required = near.join("config-dir");
        let registrations = collect_watch_registrations(&[file_target(&required)]);
        let mut watcher = RestrictedWatcher {
            denied: base.ancestors().map(Path::to_path_buf).collect(),
            ..RestrictedWatcher::default()
        };

        // Startup and refresh use the same registration policy.
        for _ in 0..2 {
            for registration in &registrations {
                registration.register(&mut watcher)?;
            }
            assert!(watcher.watched.contains(&required));
            assert!(watcher.watched.contains(&near));
        }
        Ok(())
    }

    #[test]
    fn a_required_path_also_used_as_an_ancestor_still_reports_watch_failures() {
        let base = std::env::temp_dir().join("tier-registration-test");
        for targets in [
            vec![file_target(&base), file_target(&base.join("nested"))],
            vec![file_target(&base.join("nested")), file_target(&base)],
        ] {
            let registrations = collect_watch_registrations(&targets);
            let mut watcher = RestrictedWatcher {
                denied: BTreeSet::from([base.clone()]),
                ..RestrictedWatcher::default()
            };
            assert!(
                registrations
                    .iter()
                    .any(|registration| registration.register(&mut watcher).is_err())
            );
        }
    }
}
