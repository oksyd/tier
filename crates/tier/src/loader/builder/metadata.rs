use crate::{ConfigMetadata, TierMetadata};

use super::super::{ConfigLoader, UnknownFieldPolicy, secret_path::SecretPathSpec};

impl<T> ConfigLoader<T> {
    /// Marks a dot-delimited path as sensitive for report redaction.
    #[must_use]
    pub fn secret_path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        if !path.is_empty() && path != "." {
            self.secret_paths.insert(SecretPathSpec::new(path));
        }
        self
    }

    /// Applies explicit field metadata to the loader.
    ///
    /// This is the manual alternative to [`ConfigLoader::derive_metadata`].
    /// Use it when you want env overrides, secrets, merge policies, or
    /// declared validations without enabling the `derive` feature.
    #[must_use]
    pub fn metadata(mut self, metadata: ConfigMetadata) -> Self {
        self.metadata.extend(metadata);
        self
    }

    /// Sets the active profile used by `{profile}` path templates.
    #[must_use]
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Applies metadata-derived secret paths for the target configuration type.
    ///
    /// This is the most ergonomic way to connect `#[derive(TierConfig)]`
    /// metadata to the loader when the `derive` feature is enabled.
    #[must_use]
    pub fn derive_metadata(self) -> Self
    where
        T: TierMetadata,
    {
        self.metadata(T::metadata())
    }

    /// Sets the unknown field policy.
    #[must_use]
    pub fn unknown_field_policy(mut self, policy: UnknownFieldPolicy) -> Self {
        self.unknown_field_policy = policy;
        self
    }

    /// Allows unknown fields without warnings.
    #[must_use]
    pub fn allow_unknown_fields(self) -> Self {
        self.unknown_field_policy(UnknownFieldPolicy::Allow)
    }

    /// Allows unknown fields and records warnings.
    #[must_use]
    pub fn warn_unknown_fields(self) -> Self {
        self.unknown_field_policy(UnknownFieldPolicy::Warn)
    }

    /// Rejects unknown fields with an error.
    #[must_use]
    pub fn deny_unknown_fields(self) -> Self {
        self.unknown_field_policy(UnknownFieldPolicy::Deny)
    }
}
