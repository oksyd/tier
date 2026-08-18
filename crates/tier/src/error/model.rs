use std::fmt::{self, Display, Formatter};

use crate::loader::SourceTrace;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Information about an unknown configuration path discovered during loading.
pub struct UnknownField {
    /// Dot-delimited path that was not recognized.
    pub path: String,
    /// Most recent source that contributed the unknown path, when known.
    pub source: Option<SourceTrace>,
    /// Best-effort suggestion for the intended path.
    pub suggestion: Option<String>,
}

impl UnknownField {
    /// Creates an unknown field description for a path.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: None,
            suggestion: None,
        }
    }

    /// Attaches source information.
    #[must_use]
    pub fn with_source(mut self, source: Option<SourceTrace>) -> Self {
        self.source = source;
        self
    }

    /// Attaches a best-effort suggestion.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: Option<String>) -> Self {
        self.suggestion = suggestion;
        self
    }
}

impl Display for UnknownField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "unknown field `{}`", self.path)?;
        if let Some(source) = &self.source {
            write!(f, " from {source}")?;
        }
        if let Some(suggestion) = &self.suggestion {
            write!(f, "; did you mean `{suggestion}`?")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Line and column information for parse diagnostics.
pub struct LineColumn {
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}

impl Display for LineColumn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}
