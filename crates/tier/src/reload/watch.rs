use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

#[cfg(feature = "watch")]
use super::NativeWatcher;
use super::sync::mutex_lock;
use super::{PollingWatcher, ReloadEvent, ReloadHandle, ReloadOptions};
#[cfg(feature = "watch")]
use crate::ConfigError;

impl<T> ReloadHandle<T>
where
    T: Send + Sync + 'static,
{
    /// Returns the most recent reload error, if any.
    #[must_use]
    pub fn last_error(&self) -> Option<String> {
        mutex_lock(&self.last_error).clone()
    }

    /// Subscribes to future reload events.
    pub fn subscribe(&self) -> Receiver<ReloadEvent> {
        let (tx, rx) = mpsc::channel();
        mutex_lock(&self.subscribers).push(tx);
        rx
    }

    /// Starts a polling watcher that reloads when any watched file changes.
    pub fn start_polling<I, P>(&self, paths: I, interval: Duration) -> PollingWatcher
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.start_polling_with_options(paths, interval, ReloadOptions::default())
    }

    /// Starts a polling watcher with explicit reload behavior options.
    pub fn start_polling_with_options<I, P>(
        &self,
        paths: I,
        interval: Duration,
        options: ReloadOptions,
    ) -> PollingWatcher
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        PollingWatcher::spawn(
            self.clone(),
            paths.into_iter().map(Into::into).collect(),
            interval,
            options,
        )
    }

    #[cfg(feature = "watch")]
    /// Starts a native filesystem watcher with event debouncing.
    ///
    /// File paths are watched through their parent directories so atomic
    /// replace-and-rename writes still trigger a reload.
    pub fn start_native<I, P>(
        &self,
        paths: I,
        debounce: Duration,
    ) -> Result<NativeWatcher, ConfigError>
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.start_native_with_options(paths, debounce, ReloadOptions::default())
    }

    /// Starts a native filesystem watcher with explicit reload behavior options.
    #[cfg(feature = "watch")]
    pub fn start_native_with_options<I, P>(
        &self,
        paths: I,
        debounce: Duration,
        options: ReloadOptions,
    ) -> Result<NativeWatcher, ConfigError>
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        NativeWatcher::spawn(
            self.clone(),
            paths.into_iter().map(Into::into).collect(),
            debounce,
            options,
        )
    }
}
