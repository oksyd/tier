use serde::Serialize;
use serde::de::DeserializeOwned;

use super::super::{
    ConfigLoader, schema_secrets::schema_secret_paths, secret_path::SecretPathSpec,
};

impl<T> ConfigLoader<T>
where
    T: Serialize + DeserializeOwned + schemars::JsonSchema,
{
    /// Discovers secret paths from the target type's JSON Schema.
    #[must_use]
    pub fn discover_secret_paths_from_schema(mut self) -> Self {
        for path in schema_secret_paths::<T>() {
            self.secret_paths.insert(SecretPathSpec::new(path));
        }
        self
    }
}
