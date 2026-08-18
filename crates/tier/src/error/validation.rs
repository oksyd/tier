use std::fmt::{self, Display, Formatter};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// A single validation failure returned by a validator hook.
pub struct ValidationError {
    /// Dot-delimited configuration path associated with the failure.
    pub path: String,
    /// Additional paths related to the failure, used for cross-field validations.
    pub related_paths: Vec<String>,
    /// Human-readable failure message.
    pub message: String,
    /// Optional rule identifier for machine-readable consumers.
    pub rule: Option<String>,
    /// Optional expected value associated with the failed rule.
    pub expected: Option<Value>,
    /// Optional actual value observed during validation.
    pub actual: Option<Value>,
    /// Optional machine-readable tags for downstream consumers.
    pub tags: Vec<String>,
}

impl ValidationError {
    /// Creates a new validation error.
    #[must_use]
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            related_paths: Vec::new(),
            message: message.into(),
            rule: None,
            expected: None,
            actual: None,
            tags: Vec::new(),
        }
    }

    /// Attaches a machine-readable rule identifier.
    #[must_use]
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Attaches related paths for cross-field validation failures.
    #[must_use]
    pub fn with_related_paths<I, S>(mut self, related_paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.related_paths = related_paths.into_iter().map(Into::into).collect();
        self
    }

    /// Attaches the expected value for the failed rule.
    #[must_use]
    pub fn with_expected(mut self, expected: Value) -> Self {
        self.expected = Some(expected);
        self
    }

    /// Attaches the actual value observed during validation.
    #[must_use]
    pub fn with_actual(mut self, actual: Value) -> Self {
        self.actual = Some(actual);
        self
    }

    /// Attaches machine-readable tags for downstream consumers.
    #[must_use]
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// A collection of validation failures returned by a validator hook.
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Creates an empty validation error collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a collection containing a single validation error.
    #[must_use]
    pub fn from_error(error: ValidationError) -> Self {
        Self {
            errors: vec![error],
        }
    }

    /// Creates a collection containing a single message-based validation error.
    #[must_use]
    pub fn from_message(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::from_error(ValidationError::new(path, message))
    }

    /// Appends a validation error.
    pub fn push(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Appends multiple validation errors.
    pub fn extend<I>(&mut self, errors: I)
    where
        I: IntoIterator<Item = ValidationError>,
    {
        self.errors.extend(errors);
    }

    /// Returns `true` when the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the number of validation errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Consumes the collection into a vector.
    pub fn into_vec(self) -> Vec<ValidationError> {
        self.errors
    }

    /// Returns an iterator over validation errors.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter()
    }
}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl Display for ValidationErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (index, error) in self.errors.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "- {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}
