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
        // This set is for redacting preparation errors. Validate registrations
        // against all input layers in finish, where later container shapes are known.
        let mut secrets = std::collections::BTreeSet::new();
        for spec in pending {
            secrets.insert(spec.path().to_owned());
            if let Ok((path, _)) = self.metadata.canonicalize_alias_path_with_array_segments(
                spec.path(),
                spec.explicit_array_segments(),
            ) {
                secrets.insert(path);
            }
            if let Ok(defaults) = &self.defaults
                && let Ok(paths) = super::canonical::canonicalize_secret_paths_against_value(
                    &std::collections::BTreeSet::from([spec]),
                    defaults,
                    &self.metadata,
                )
            {
                secrets.extend(paths);
            }
        }
        let metadata = self.metadata.clone();
        LoadSession::prepare(self)
            .and_then(LoadSession::load)
            .map_err(|error| secrets::redact_source_error(error, &secrets, &metadata))
    }
}
