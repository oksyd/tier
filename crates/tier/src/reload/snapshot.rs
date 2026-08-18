use std::sync::Arc;

use crate::{ConfigReport, LoadedConfig};

use super::ReloadHandle;
use super::sync::read_lock;

impl<T> ReloadHandle<T>
where
    T: Send + Sync + 'static,
{
    /// Returns a shared immutable snapshot of the current configuration and report.
    ///
    /// Cloning the returned [`Arc`] is constant time. A successful reload swaps in
    /// a new snapshot without invalidating snapshots already held by readers.
    #[must_use]
    pub fn snapshot(&self) -> Arc<LoadedConfig<T>> {
        Arc::clone(&read_lock(&self.state))
    }

    /// Returns a cloned copy of the current configuration report.
    #[must_use]
    pub fn report(&self) -> ConfigReport {
        self.snapshot().report().clone()
    }
}

impl<T> ReloadHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Returns a cloned copy of the current configuration value.
    #[must_use]
    pub fn config(&self) -> T {
        self.snapshot().config().clone()
    }
}
