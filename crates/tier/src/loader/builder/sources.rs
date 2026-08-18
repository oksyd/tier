use std::path::PathBuf;

use super::super::{ArgsSource, ConfigLoader, EnvSource, FileSource, Layer, PendingCustomLayer};

impl<T> ConfigLoader<T> {
    /// Adds a required configuration file.
    #[must_use]
    pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
        self.files.push(FileSource::new(path));
        self
    }

    /// Adds an optional configuration file.
    #[must_use]
    pub fn optional_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.files.push(FileSource::optional(path));
        self
    }

    /// Adds a custom file source definition.
    #[must_use]
    pub fn with_file(mut self, file: FileSource) -> Self {
        self.files.push(file);
        self
    }

    /// Adds an environment variable source.
    #[must_use]
    pub fn env(mut self, source: EnvSource) -> Self {
        self.env_sources.push(source);
        self
    }

    /// Adds CLI overrides from an [`ArgsSource`].
    #[must_use]
    pub fn args(mut self, source: ArgsSource) -> Self {
        self.args_source = Some(source);
        self
    }

    /// Adds a custom serializable layer.
    pub fn layer(mut self, layer: Layer) -> Self {
        self.custom_layers
            .push(PendingCustomLayer::Immediate(layer));
        self
    }
}
