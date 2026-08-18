use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};

use crate::{ConfigError, LoadedConfig};

mod core;
mod model;
#[cfg(feature = "watch")]
mod native;
mod polling;
mod snapshot;
mod summary;
mod sync;
mod watch;

pub use self::model::{ReloadEvent, ReloadFailure, ReloadFailurePolicy, ReloadOptions};
#[cfg(feature = "watch")]
pub use self::native::NativeWatcher;
pub use self::polling::PollingWatcher;
pub use self::summary::{ConfigChange, ReloadSummary};

type LoaderFn<T> = dyn Fn() -> Result<LoadedConfig<T>, ConfigError> + Send + Sync + 'static;

/// Thread-safe holder for the active configuration and reload logic.
///
/// `ReloadHandle<T>` keeps the most recent successful [`LoadedConfig`] in
/// memory and reuses the same loader closure for subsequent reloads. Failed
/// reloads never replace the active configuration, which makes it suitable for
/// long-running services.
///
/// # Examples
///
/// ```no_run
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigLoader, ReloadHandle};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// impl Default for AppConfig {
///     fn default() -> Self {
///         Self { port: 3000 }
///     }
/// }
///
/// let handle = ReloadHandle::new(|| ConfigLoader::new(AppConfig::default()).load())?;
/// let snapshot = handle.snapshot();
/// assert_eq!(snapshot.port, 3000);
/// # Ok::<(), tier::ConfigError>(())
/// ```
pub struct ReloadHandle<T> {
    state: Arc<RwLock<Arc<LoadedConfig<T>>>>,
    loader: Arc<LoaderFn<T>>,
    reload_gate: Arc<Mutex<()>>,
    last_error: Arc<Mutex<Option<String>>>,
    subscribers: Arc<Mutex<Vec<Sender<ReloadEvent>>>>,
}
