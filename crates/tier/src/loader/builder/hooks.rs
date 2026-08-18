use std::fmt::Display;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ValidationErrors;

use super::super::{ConfigLoader, NamedNormalizer, NamedValidator};

impl<T> ConfigLoader<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Registers a normalization hook applied after merge and before validation.
    #[must_use]
    pub fn normalizer<F, E>(mut self, name: impl Into<String>, normalizer: F) -> Self
    where
        F: Fn(&mut T) -> Result<(), E> + Send + Sync + 'static,
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
