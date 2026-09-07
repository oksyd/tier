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
    T: DeserializeOwned,
{
    /// Loads configuration from all configured layers.
    pub fn load(self) -> Result<LoadedConfig<T>, ConfigError> {
        let pending =
            secrets::normalize_secret_registration_paths(&self.secret_paths, &self.metadata)?;
        let secrets = match &self.defaults {
            Ok(defaults) => super::canonical::canonicalize_secret_paths_against_value(
                &pending,
                defaults,
                &self.metadata,
            )?,
            Err(_) => pending.iter().map(|spec| spec.path().to_owned()).collect(),
        };
        let metadata = self.metadata.clone();
        LoadSession::prepare(self)
            .and_then(LoadSession::load)
            .map_err(|error| secrets::redact_source_error(error, &secrets, &metadata))
    }
}
