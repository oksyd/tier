use crate::loader::SourceKind;

use super::super::{EffectiveSourcePolicy, FieldMetadata};

impl EffectiveSourcePolicy {
    pub(crate) fn apply_field(&mut self, field: &FieldMetadata) {
        if let Some(allowed_sources) = &field.allowed_sources {
            self.allowed_sources = Some(allowed_sources.clone());
        }
        if let Some(denied_sources) = &field.denied_sources {
            self.denied_sources = Some(denied_sources.clone());
        }
    }

    pub(crate) fn source_kind_allowed(&self, kind: SourceKind) -> bool {
        self.allowed_sources
            .as_ref()
            .is_none_or(|allowed_sources| allowed_sources.contains(&kind))
    }

    pub(crate) fn source_kind_denied(&self, kind: SourceKind) -> bool {
        self.denied_sources
            .as_ref()
            .is_some_and(|denied_sources| denied_sources.contains(&kind))
    }

    pub(crate) fn allowed_sources_vec(&self) -> Vec<SourceKind> {
        self.allowed_sources
            .as_ref()
            .map(|allowed_sources| allowed_sources.iter().copied().collect())
            .unwrap_or_default()
    }

    pub(crate) fn denied_sources_vec(&self) -> Vec<SourceKind> {
        self.denied_sources
            .as_ref()
            .map(|denied_sources| denied_sources.iter().copied().collect())
            .unwrap_or_default()
    }
}
