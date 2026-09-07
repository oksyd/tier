use crate::{ConfigError, ConfigMetadata};

use super::{Layer, merge::MergeEffects};

pub(super) fn enforce_source_policies(
    layer: &Layer,
    effects: &MergeEffects,
    metadata: &ConfigMetadata,
) -> Result<(), ConfigError> {
    // Removed descendants are writes by the replacing source, but have no
    // overlay value and must remain separate from recorded layer entries.
    for (path, trace) in layer.entries.iter().chain(&effects.removed_entries) {
        enforce_source_policy(path, trace, metadata)?;
    }
    Ok(())
}

pub(super) fn enforce_source_policy(
    path: &str,
    trace: &super::SourceTrace,
    metadata: &ConfigMetadata,
) -> Result<(), ConfigError> {
    let Some(policy) = metadata.effective_source_policy_for(path) else {
        return Ok(());
    };

    if !policy.source_kind_allowed(trace.kind) {
        return Err(ConfigError::SourcePolicyViolation {
            path: path.to_owned(),
            trace: trace.clone(),
            allowed_sources: policy.allowed_sources_vec().into_boxed_slice(),
            denied_sources: Vec::new().into_boxed_slice(),
        });
    }

    if policy.source_kind_denied(trace.kind) {
        return Err(ConfigError::SourcePolicyViolation {
            path: path.to_owned(),
            trace: trace.clone(),
            allowed_sources: policy.allowed_sources_vec().into_boxed_slice(),
            denied_sources: policy.denied_sources_vec().into_boxed_slice(),
        });
    }
    Ok(())
}
