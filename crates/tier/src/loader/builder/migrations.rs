use serde::Serialize;
use serde::de::DeserializeOwned;

use super::super::{ConfigLoader, ConfigMigration};

impl<T> ConfigLoader<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Declares the configuration version path and the newest version this
    /// loader understands.
    #[must_use]
    pub fn config_version(mut self, path: impl Into<String>, current_version: u32) -> Self {
        self.config_version = Some((path.into(), current_version));
        self
    }

    /// Registers a migration rule applied before deserialization.
    #[must_use]
    pub fn migration(mut self, migration: ConfigMigration) -> Self {
        self.migrations.push(migration);
        self
    }
}
