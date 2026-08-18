use std::fmt::{self, Display, Formatter};

use crate::error::{UnknownField, ValidationError};
use crate::loader::SourceTrace;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Information about a deprecated configuration path used during loading.
pub struct DeprecatedField {
    /// Dot-delimited deprecated path.
    pub path: String,
    /// Most recent source that contributed the deprecated path, when known.
    pub source: Option<SourceTrace>,
    /// Optional migration note or replacement guidance.
    pub note: Option<String>,
}

impl DeprecatedField {
    /// Creates a deprecated field diagnostic for a path.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: None,
            note: None,
        }
    }

    /// Attaches source information.
    #[must_use]
    pub fn with_source(mut self, source: Option<SourceTrace>) -> Self {
        self.source = source;
        self
    }

    /// Attaches an optional migration note.
    #[must_use]
    pub fn with_note(mut self, note: Option<String>) -> Self {
        self.note = note;
        self
    }
}

impl Display for DeprecatedField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "deprecated field `{}`", self.path)?;
        if let Some(source) = &self.source {
            write!(f, " from {source}")?;
        }
        if let Some(note) = &self.note {
            write!(f, "; {note}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// A migration step applied while upgrading configuration input.
pub struct AppliedMigration {
    /// Stable migration kind.
    pub kind: String,
    /// Source version that triggered the migration.
    pub from_version: u32,
    /// Target version this migration belongs to.
    pub to_version: u32,
    /// Original path affected by the migration.
    pub from_path: String,
    /// Replacement path when the migration renames a field.
    pub to_path: Option<String>,
    /// Optional operator-facing note.
    pub note: Option<String>,
}

impl Display for AppliedMigration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.to_path {
            Some(to_path) => {
                write!(
                    f,
                    "{} {} -> {} (v{} -> v{})",
                    self.kind, self.from_path, to_path, self.from_version, self.to_version
                )?;
            }
            None => {
                write!(
                    f,
                    "{} {} (v{} -> v{})",
                    self.kind, self.from_path, self.from_version, self.to_version
                )?;
            }
        }
        if let Some(note) = &self.note {
            write!(f, "; {note}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Non-fatal issues surfaced while loading configuration.
pub enum ConfigWarning {
    /// A path was present in input but not recognized by the target type.
    UnknownField(UnknownField),
    /// A deprecated path was used by one of the configured sources.
    DeprecatedField(DeprecatedField),
    /// A declarative validation was configured as warning-level.
    Validation(ValidationError),
}

impl Display for ConfigWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownField(field) => Display::fmt(field, f),
            Self::DeprecatedField(field) => Display::fmt(field, f),
            Self::Validation(error) => Display::fmt(error, f),
        }
    }
}
