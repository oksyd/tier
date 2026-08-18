use std::fmt::{self, Display, Formatter};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
/// Kind of source that contributed configuration values.
pub enum SourceKind {
    /// Values originating from in-code defaults.
    Default,
    /// Values originating from configuration files.
    File,
    /// Values originating from environment variables.
    Environment,
    /// Values originating from CLI overrides.
    Arguments,
    /// Values originating from normalization hooks.
    Normalization,
    /// Values originating from a custom layer.
    Custom,
}

impl Display for SourceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::File => write!(f, "file"),
            Self::Environment => write!(f, "env"),
            Self::Arguments => write!(f, "cli"),
            Self::Normalization => write!(f, "normalize"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Human-readable description of where a configuration value came from.
pub struct SourceTrace {
    /// High-level source category.
    pub kind: SourceKind,
    /// Source name, such as a file path or environment variable name.
    pub name: String,
    /// Optional location inside the source, when available.
    pub location: Option<String>,
}

impl SourceTrace {
    pub(super) fn new(kind: SourceKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            location: None,
        }
    }
}

impl Display for SourceTrace {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(location) if self.name.is_empty() => write!(f, "{}({location})", self.kind),
            Some(location) => write!(f, "{}({}:{location})", self.kind, self.name),
            None if self.name.is_empty() => write!(f, "{}", self.kind),
            None => write!(f, "{}({})", self.kind, self.name),
        }
    }
}
