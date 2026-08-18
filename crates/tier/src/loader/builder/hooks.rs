use std::fmt::Display;

use serde_json::Value;

use crate::error::ValidationErrors;

use super::super::{ConfigLoader, NamedNormalizer, NamedValidator};

impl<T> ConfigLoader<T> {
    /// Registers a normalization hook applied to the merged configuration
    /// document before typed deserialization and validation.
    #[must_use]
    pub fn normalizer<F, E>(mut self, name: impl Into<String>, normalizer: F) -> Self
    where
        F: Fn(&mut Value) -> Result<(), E> + Send + Sync + 'static,
        E: Display,
    {
        self.normalizers.push(NamedNormalizer {
            name: name.into(),
            run: Box::new(move |config| normalizer(config).map_err(|error| error.to_string())),
        });
        self
    }

    /// Registers a validation hook applied after normalization.
    #[must_use]
    pub fn validator<F>(mut self, name: impl Into<String>, validator: F) -> Self
    where
        F: Fn(&T) -> Result<(), ValidationErrors> + Send + Sync + 'static,
    {
        self.validators.push(NamedValidator {
            name: name.into(),
            run: Box::new(validator),
        });
        self
    }
}
