use std::sync::Arc;

use crate::{ConfigError, LoadedConfig};

use super::summary::build_reload_summary;
use super::sync::{mutex_lock, read_lock, write_lock};
use super::{
    ReloadEvent, ReloadFailure, ReloadFailurePolicy, ReloadHandle, ReloadOptions, ReloadSummary,
};

impl<T> Clone for ReloadHandle<T> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            loader: Arc::clone(&self.loader),
            reload_gate: Arc::clone(&self.reload_gate),
            last_error: Arc::clone(&self.last_error),
            subscribers: Arc::clone(&self.subscribers),
        }
    }
}

impl<T> ReloadHandle<T>
where
    T: Send + Sync + 'static,
{
    /// Creates a reload handle from a loader closure and performs the initial load.
    pub fn new<F>(loader: F) -> Result<Self, ConfigError>
    where
        F: Fn() -> Result<LoadedConfig<T>, ConfigError> + Send + Sync + 'static,
    {
        let initial = loader()?;
        Ok(Self {
            state: Arc::new(std::sync::RwLock::new(Arc::new(initial))),
            loader: Arc::new(loader),
            reload_gate: Arc::new(std::sync::Mutex::new(())),
            last_error: Arc::new(std::sync::Mutex::new(None)),
            subscribers: Arc::new(std::sync::Mutex::new(Vec::new())),
        })
    }

    /// Attempts to reload configuration, preserving the previous state on failure.
    pub fn reload(&self) -> Result<(), ConfigError> {
        self.reload_with_options(&ReloadOptions {
            emit_unchanged: true,
            ..ReloadOptions::default()
        })
        .map(|_| ())
    }

    /// Attempts to reload configuration and returns a structured diff summary.
    pub fn reload_detailed(&self) -> Result<ReloadSummary, ConfigError> {
        self.reload_with_options(&ReloadOptions {
            emit_unchanged: true,
            ..ReloadOptions::default()
        })
    }

    pub(in crate::reload) fn reload_with_options(
        &self,
        options: &ReloadOptions,
    ) -> Result<ReloadSummary, ConfigError> {
        // Reload is a transaction, not merely a synchronized final write. Holding
        // this gate ensures the diff is computed against the state that is
        // actually replaced and prevents a slower stale load from winning later.
        let _reload_guard = mutex_lock(&self.reload_gate);
        let before = Arc::clone(&read_lock(&self.state));
        let before_report = before.report();
        let before_raw = before_report.final_value().clone();
        let before_redacted = before_report.redacted_value();

        match (self.loader)() {
            Ok(next) => {
                let after_raw = next.report().final_value().clone();
                let after_redacted = next.report().redacted_value();
                let summary = build_reload_summary(
                    &before_raw,
                    &after_raw,
                    &before_redacted,
                    &after_redacted,
                );
                *write_lock(&self.state) = Arc::new(next);
                *mutex_lock(&self.last_error) = None;
                if options.emit_unchanged || summary.had_changes {
                    self.emit_event(ReloadEvent::Applied(summary.clone()));
                }
                Ok(summary)
            }
            Err(error) => {
                self.record_error_message(error.to_string());
                self.emit_event(ReloadEvent::Rejected(ReloadFailure {
                    error: error.to_string(),
                    last_good_retained: true,
                    watcher_stopped: matches!(options.on_error, ReloadFailurePolicy::StopWatcher),
                }));
                Err(error)
            }
        }
    }
}

impl<T> ReloadHandle<T> {
    pub(in crate::reload) fn record_error_message(&self, message: String) {
        *mutex_lock(&self.last_error) = Some(message);
    }

    pub(in crate::reload) fn emit_event(&self, event: ReloadEvent) {
        let mut subscribers = mutex_lock(&self.subscribers);
        subscribers.retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }
}
