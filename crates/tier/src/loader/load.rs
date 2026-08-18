use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ConfigError;

mod runtime;
mod secrets;
mod session;
mod unknown_policy;
mod validate;

use self::session::LoadSession;
use super::{ConfigLoader, LoadedConfig};

impl<T> ConfigLoader<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Loads configuration from all configured layers.
    pub fn load(self) -> Result<LoadedConfig<T>, ConfigError> {
        LoadSession::prepare(self)?.load()
    }
}
