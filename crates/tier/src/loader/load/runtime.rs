use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::error::ConfigError;
use crate::metadata::ConfigMetadata;
use crate::report::ConfigReport;

use crate::loader::canonical::{
    canonicalize_metadata_against_value, canonicalize_secret_paths_against_value,
};
use crate::loader::indexed_array::validate_indexed_array_paths;
use crate::loader::merge::{ensure_root_object, merge_values};
use crate::loader::path::ensure_path_safe_keys;
use crate::loader::policy::enforce_source_policies;
use crate::loader::secret_path::SecretPathSpec;
use crate::loader::trace::{record_deprecation_warnings, record_diff_steps, record_layer_steps};
use crate::loader::{Layer, NamedNormalizer, SourceKind, SourceTrace};

pub(super) struct MergedLayers {
    pub(super) merged: Value,
    pub(super) string_coercion_paths: BTreeSet<String>,
}

pub(super) struct RuntimeMetadata {
    pub(super) alias_overrides: BTreeMap<String, String>,
    pub(super) secret_paths: BTreeSet<String>,
}

pub(super) fn merge_layers_into_report(
    layers: Vec<Layer>,
    defaults_value: Value,
    metadata: &ConfigMetadata,
    secret_paths: &BTreeSet<String>,
    report: &mut ConfigReport,
) -> Result<MergedLayers, ConfigError> {
    let mut merged = defaults_value;
    ensure_root_object(&merged)?;

    let mut string_coercion_paths = BTreeSet::new();
    for layer in layers {
        string_coercion_paths.extend(layer.coercible_string_paths.iter().cloned());
        validate_indexed_array_paths(&merged, &layer)?;
        enforce_source_policies(&layer, metadata)?;
        report.record_source(layer.trace.clone());
        record_layer_steps(report, &layer, secret_paths);
        record_deprecation_warnings(report, &layer, metadata);
        if !matches!(layer.trace.kind, SourceKind::Default) {
            merge_values(
                &mut merged,
                layer.value,
                "",
                metadata,
                &layer.indexed_array_paths,
                &layer.direct_array_paths,
            )?;
        }
    }

    Ok(MergedLayers {
        merged,
        string_coercion_paths,
    })
}

pub(super) fn run_normalizers(
    normalizers: Vec<NamedNormalizer>,
    config: &mut Value,
    metadata: &mut ConfigMetadata,
    pending_secret_paths: &BTreeSet<SecretPathSpec>,
    runtime_metadata: &mut RuntimeMetadata,
    report: &mut ConfigReport,
) -> Result<(), ConfigError> {
    for normalizer in normalizers {
        let before = config.clone();
        (normalizer.run)(config).map_err(|message| ConfigError::Normalize {
            name: normalizer.name.clone(),
            message,
        })?;
        let after = config.clone();
        ensure_root_object(&after)?;
        ensure_path_safe_keys(&after, "")?;

        *metadata = canonicalize_metadata_against_value(metadata, &after)?;
        runtime_metadata.alias_overrides = metadata.alias_lookup_overrides()?;
        runtime_metadata.secret_paths =
            canonicalize_secret_paths_against_value(pending_secret_paths, &after, metadata)?;

        let trace = SourceTrace::new(SourceKind::Normalization, normalizer.name.clone());
        report.record_source(trace.clone());
        record_diff_steps(
            report,
            &before,
            &after,
            &trace,
            &runtime_metadata.secret_paths,
        );
    }

    Ok(())
}
