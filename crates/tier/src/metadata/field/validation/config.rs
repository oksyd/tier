use super::super::super::{FieldMetadata, ValidationLevel};

impl FieldMetadata {
    /// Overrides the human-readable message for a validation rule.
    #[must_use]
    pub fn validation_message(
        mut self,
        rule_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.validation_configs
            .entry(rule_code.into())
            .or_default()
            .message = Some(message.into());
        self
    }

    /// Sets the runtime level for a validation rule.
    #[must_use]
    pub fn validation_level(
        mut self,
        rule_code: impl Into<String>,
        level: ValidationLevel,
    ) -> Self {
        self.validation_configs
            .entry(rule_code.into())
            .or_default()
            .level = level;
        self
    }

    /// Attaches machine-readable tags to a validation rule.
    #[must_use]
    pub fn validation_tags<I, S>(mut self, rule_code: impl Into<String>, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.validation_configs
            .entry(rule_code.into())
            .or_default()
            .tags = tags.into_iter().map(Into::into).collect();
        self
    }
}
