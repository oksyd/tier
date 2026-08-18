use std::collections::BTreeMap;
use std::path::PathBuf;
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
        let join = thread::spawn(move || {
            let mut seen = collect_mtimes(&paths);
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

fn collect_mtimes(paths: &[PathBuf]) -> BTreeMap<PathBuf, Option<SystemTime>> {
    let mut mtimes = BTreeMap::new();
    for path in paths {
        collect_mtimes_recursive(path, &mut mtimes);
    }
    mtimes
}

fn collect_mtimes_recursive(path: &PathBuf, mtimes: &mut BTreeMap<PathBuf, Option<SystemTime>>) {
    let metadata = std::fs::symlink_metadata(path).ok();
    let mtime = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok());
    mtimes.insert(path.clone(), mtime);

    let is_dir = metadata
        .as_ref()
        .is_some_and(|metadata| metadata.file_type().is_dir());
    if !is_dir {
        return;
    }

    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        collect_mtimes_recursive(&entry.path(), mtimes);
    }
}
