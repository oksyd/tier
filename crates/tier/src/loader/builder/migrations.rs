use super::super::{ConfigLoader, ConfigMigration};

impl<T> ConfigLoader<T> {
    /// Declares the configuration version path and the newest version this
    /// loader understands.
    ///
    /// Paths accept field aliases and bracket array indices. Migrations run
    /// after merging sources and before normalization and deserialization.
    #[must_use]
    pub fn config_version(mut self, path: impl Into<String>, current_version: u32) -> Self {
        self.config_version = Some((path.into(), current_version));
        self
    }

    /// Registers a migration rule applied before deserialization.
    ///
    /// Requires [`Self::config_version`]. Rules run in increasing version order,
    /// preserving registration order for rules introduced in the same version.
    #[must_use]
    pub fn migration(mut self, migration: ConfigMigration) -> Self {
        self.migrations.push(migration);
        self
    }
}
