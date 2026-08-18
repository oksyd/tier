use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// Policy applied when unknown configuration paths are discovered.
pub enum UnknownFieldPolicy {
    /// Accept unknown fields silently.
    Allow,
    /// Accept unknown fields but emit warnings.
    Warn,
    #[default]
    /// Reject unknown fields with an error.
    Deny,
}

impl Display for UnknownFieldPolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Warn => write!(f, "warn"),
            Self::Deny => write!(f, "deny"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
/// Policy used when both sides of a rename migration contain explicit values.
pub enum MigrationConflictPolicy {
    /// Reject the ambiguous configuration instead of choosing a value implicitly.
    #[default]
    Error,
    /// Remove the legacy path and retain the explicitly configured target value.
    KeepTarget,
    /// Replace the explicitly configured target value with the legacy value.
    OverwriteTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Migration action applied when upgrading older configuration payloads.
pub enum ConfigMigrationKind {
    /// Renames one configuration path to another.
    Rename {
        /// Original path used by older configs.
        from: String,
        /// Replacement path used by newer configs.
        to: String,
        /// Behavior when both paths contain explicit values.
        #[serde(default)]
        conflict_policy: MigrationConflictPolicy,
    },
    /// Removes a configuration path that is no longer supported.
    Remove {
        /// Path removed from newer configs.
        path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Declarative migration rule applied to loaded configuration values.
pub struct ConfigMigration {
    /// Version introduced by this migration rule.
    pub since_version: u32,
    /// Concrete migration action.
    pub kind: ConfigMigrationKind,
    /// Optional operator-facing migration note.
    pub note: Option<String>,
}

impl ConfigMigration {
    /// Creates a rename migration from `from` to `to`.
    #[must_use]
    pub fn rename(from: impl Into<String>, to: impl Into<String>, since_version: u32) -> Self {
        Self::rename_with_policy(from, to, since_version, MigrationConflictPolicy::Error)
    }

    /// Creates a rename migration with an explicit conflict policy.
    #[must_use]
    pub fn rename_with_policy(
        from: impl Into<String>,
        to: impl Into<String>,
        since_version: u32,
        conflict_policy: MigrationConflictPolicy,
    ) -> Self {
        Self {
            since_version,
            kind: ConfigMigrationKind::Rename {
                from: from.into(),
                to: to.into(),
                conflict_policy,
            },
            note: None,
        }
    }

    /// Creates a removal migration for `path`.
    #[must_use]
    pub fn remove(path: impl Into<String>, since_version: u32) -> Self {
        Self {
            since_version,
            kind: ConfigMigrationKind::Remove { path: path.into() },
            note: None,
        }
    }

    /// Attaches an operator-facing migration note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}
