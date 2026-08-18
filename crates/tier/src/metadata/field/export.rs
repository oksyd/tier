#[cfg(feature = "schema")]
use serde_json::Value;

use super::super::FieldMetadata;
#[cfg(feature = "schema")]
use super::FieldValidationExport;

impl FieldMetadata {
    #[cfg(feature = "schema")]
    pub(crate) fn allowed_source_names(&self) -> Vec<String> {
        self.allowed_sources
            .as_ref()
            .map(|allowed_sources| allowed_sources.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .map(|source| source.to_string())
            .collect()
    }

    #[cfg(feature = "schema")]
    pub(crate) fn denied_source_names(&self) -> Vec<String> {
        self.denied_sources
            .as_ref()
            .map(|denied_sources| denied_sources.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .map(|source| source.to_string())
            .collect()
    }

    #[cfg(feature = "schema")]
    pub(crate) fn validation_export(&self) -> FieldValidationExport {
        let mut export = FieldValidationExport::default();
        for (rule_code, config) in &self.validation_configs {
            export.levels.insert(rule_code.clone(), config.level);
            if let Some(message) = &config.message {
                export.messages.insert(rule_code.clone(), message.clone());
            }
            if !config.tags.is_empty() {
                export.tags.insert(rule_code.clone(), config.tags.clone());
            }
        }
        export
    }

    #[cfg(feature = "schema")]
    pub(crate) fn validation_config_json(&self) -> Option<Value> {
        if self.validation_configs.is_empty() {
            None
        } else {
            Some(
                serde_json::to_value(&self.validation_configs)
                    .unwrap_or_else(|_| Value::Object(Default::default())),
            )
        }
    }
}
