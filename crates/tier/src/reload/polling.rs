use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use super::{ReloadFailurePolicy, ReloadHandle, ReloadOptions};

/// Handle for a background polling watcher.
pub struct PollingWatcher {
    stop: Sender<()>,
    join: Option<JoinHandle<()>>,
}

impl PollingWatcher {
    pub(super) fn spawn<T>(
        handle: ReloadHandle<T>,
        paths: Vec<PathBuf>,
        interval: Duration,
        options: ReloadOptions,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        let (stop, stop_rx) = mpsc::channel();
        let interval = effective_interval(interval);
        // Capture the baseline before returning so immediate edits are observed.
        let mut seen = collect_mtimes(&paths);
        let join = thread::spawn(move || {
            loop {
                if stop_rx.recv_timeout(interval).is_ok() {
                    return;
                }

                let current = collect_mtimes(&paths);
                if current == seen {
                    continue;
                }

                let reload_result = handle.reload_with_options(&options);
                seen = current;
                if reload_result.is_err()
                    && matches!(options.on_error, ReloadFailurePolicy::StopWatcher)
                {
                    return;
                }
            }
        });

        Self {
            stop,
            join: Some(join),
        }
    }

    /// Stops the watcher and joins the background thread.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let _ = self.stop.send(());
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for PollingWatcher {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn effective_interval(interval: Duration) -> Duration {
    interval.max(Duration::from_millis(1))
}

#[derive(PartialEq, Eq)]
struct PathState {
    modified: Option<SystemTime>,
    link: Option<LinkState>,
}

#[derive(PartialEq, Eq)]
struct LinkState {
    modified: Option<SystemTime>,
    target: Option<PathBuf>,
}

fn collect_mtimes(paths: &[PathBuf]) -> BTreeMap<PathBuf, PathState> {
    let mut mtimes = BTreeMap::new();
    let mut ancestors = BTreeSet::new();
    for path in paths {
        collect_mtimes_recursive(path, &mut mtimes, &mut ancestors);
    }
    mtimes
}

fn collect_mtimes_recursive(
    path: &Path,
    mtimes: &mut BTreeMap<PathBuf, PathState>,
    ancestors: &mut BTreeSet<PathBuf>,
) {
    let metadata = std::fs::symlink_metadata(path).ok();
    let link = metadata
        .as_ref()
        .filter(|metadata| metadata.file_type().is_symlink())
        .map(|metadata| LinkState {
            modified: metadata.modified().ok(),
            target: std::fs::read_link(path).ok(),
        });
    let metadata = if link.is_some() {
        std::fs::metadata(path).ok()
    } else {
        metadata
    };
    mtimes.insert(
        path.to_path_buf(),
        PathState {
            modified: metadata
                .as_ref()
                .and_then(|metadata| metadata.modified().ok()),
            link,
        },
    );

    let is_dir = metadata
        .as_ref()
        .is_some_and(|metadata| metadata.file_type().is_dir());
    if !is_dir {
        return;
    }

    // Follow directory links, but stop when they lead back into the current
    // traversal. Keep separate aliases so either watched path remains tracked.
    let Ok(canonical_path) = std::fs::canonicalize(path) else {
        return;
    };
    if !ancestors.insert(canonical_path.clone()) {
        return;
    }

    let Ok(entries) = std::fs::read_dir(path) else {
        ancestors.remove(&canonical_path);
        return;
    };

    for entry in entries.flatten() {
        collect_mtimes_recursive(&entry.path(), mtimes, ancestors);
    }
    ancestors.remove(&canonical_path);
}
