use crate::error::ValidationError;

use super::super::super::{FieldMetadata, ValidationLevel, ValidationRule, ValidationRuleConfig};

impl FieldMetadata {
    pub(crate) fn validation_config_for(
        &self,
        rule: &ValidationRule,
    ) -> Option<&ValidationRuleConfig> {
        self.validation_configs.get(rule.code())
    }

    pub(crate) fn validation_level_for(&self, rule: &ValidationRule) -> ValidationLevel {
        self.validation_config_for(rule)
            .map(|config| config.level)
            .unwrap_or(ValidationLevel::Error)
    }

    pub(crate) fn decorate_validation_error(
        &self,
        rule: &ValidationRule,
        mut error: ValidationError,
    ) -> ValidationError {
        if let Some(config) = self.validation_config_for(rule) {
            if let Some(message) = &config.message {
                error.message = message.clone();
            }
            if !config.tags.is_empty() {
                error = error.with_tags(config.tags.clone());
            }
        }
        error
    }
}
