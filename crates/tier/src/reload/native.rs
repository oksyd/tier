mod event;
mod target;
mod worker;

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::ConfigError;

use self::event::WatchMessage;
use self::target::{collect_watch_registrations, prepare_watch_targets};
use self::worker::run_native_watch_loop;
use super::{ReloadHandle, ReloadOptions};

/// Handle for a background native filesystem watcher.
pub struct NativeWatcher {
    watcher: Option<RecommendedWatcher>,
    stop: Option<Sender<WatchMessage>>,
    join: Option<JoinHandle<()>>,
}

impl NativeWatcher {
    pub(super) fn spawn<T>(
        handle: ReloadHandle<T>,
        paths: Vec<PathBuf>,
        debounce: Duration,
        options: ReloadOptions,
    ) -> Result<Self, ConfigError>
    where
        T: Send + Sync + 'static,
    {
        let targets = prepare_watch_targets(paths)?;
        if targets.is_empty() {
            return Err(ConfigError::Watch {
                message: "at least one path must be watched".to_owned(),
            });
        }

        let registrations = collect_watch_registrations(&targets);
        let (tx, rx) = mpsc::channel();
        let callback_tx = tx.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = callback_tx.send(WatchMessage::Event(event));
        })
        .map_err(map_watch_error)?;

        for registration in &registrations {
            watcher
                .watch(&registration.root, registration.mode())
                .map_err(map_watch_error)?;
        }

        let join =
            thread::spawn(move || run_native_watch_loop(handle, targets, rx, debounce, options));

        Ok(Self {
            watcher: Some(watcher),
            stop: Some(tx),
            join: Some(join),
        })
    }

    /// Stops the watcher and joins the background thread.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.watcher.take();
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(WatchMessage::Stop);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for NativeWatcher {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn map_watch_error(error: notify::Error) -> ConfigError {
    ConfigError::Watch {
        message: error.to_string(),
    }
}

impl target::WatchRegistration {
    fn mode(&self) -> RecursiveMode {
        if self.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        }
    }
}
