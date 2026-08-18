use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::ConfigError;
use crate::report::ConfigReport;

use super::LoadSession;
use crate::loader::LoadedConfig;
use crate::loader::canonical::{
    canonicalize_secret_paths_against_layers, canonicalize_value_paths,
};
use crate::loader::de::deserialize_with_path;
use crate::loader::load::runtime::{RuntimeMetadata, merge_layers_into_report, run_normalizers};
use crate::loader::load::secrets::normalize_secret_registration_paths;
use crate::loader::load::unknown_policy::{
    apply_unknown_field_policy, pre_deserialize_unknown_fields_error,
};
use crate::loader::load::validate::validate_loaded_config;
use crate::loader::migration_runtime::{
    apply_config_migrations, normalize_version_registration_path,
};
use crate::loader::unknown::{
    collect_known_paths, collect_known_paths_from_value, collect_suggestion_paths,
};

impl<T> LoadSession<T>
where
    T: Serialize + DeserializeOwned,
{
    pub(super) fn finish(mut self) -> Result<LoadedConfig<T>, ConfigError> {
        self.recanonicalize_metadata()?;
        let alias_overrides = self.metadata.alias_lookup_overrides()?;
        let pending_secret_paths =
            normalize_secret_registration_paths(&self.secret_paths, &self.metadata)?;
        let defaults_value =
            canonicalize_value_paths(&serde_json::to_value(&self.defaults)?, &self.metadata)?;
        let secret_paths = canonicalize_secret_paths_against_layers(
            &pending_secret_paths,
            &defaults_value,
            &self.layers,
            &self.metadata,
        )?;
        let pre_deserialize_suggestion_paths =
            self.pre_deserialize_suggestion_paths(&defaults_value);
        let mut report = ConfigReport::new(
            defaults_value.clone(),
            secret_paths.clone(),
            alias_overrides,
        );
        let merged_layers = merge_layers_into_report(
            std::mem::take(&mut self.layers),
            defaults_value,
            &self.metadata,
            &secret_paths,
            &mut report,
        )?;
        let mut merged = merged_layers.merged;
        let string_coercion_paths = merged_layers.string_coercion_paths;

        self.apply_migrations(&mut merged, &mut report)?;
        let mut config = self.deserialize_config(
            &merged,
            &report,
            &string_coercion_paths,
            &pre_deserialize_suggestion_paths,
        )?;
        self.apply_post_deserialize_unknown_policy(
            &config,
            &merged,
            &mut report,
            &string_coercion_paths,
        )?;

        let mut runtime_metadata = RuntimeMetadata {
            alias_overrides: self.metadata.alias_lookup_overrides()?,
            secret_paths,
        };
        run_normalizers(
            self.normalizers,
            &mut config,
            &mut self.metadata,
            &pending_secret_paths,
            &mut runtime_metadata,
            &mut report,
        )?;

        report.replace_runtime_metadata(
            runtime_metadata.secret_paths.clone(),
            runtime_metadata.alias_overrides.clone(),
        );
        let final_value = validate_loaded_config(
            &config,
            &self.metadata,
            &runtime_metadata.secret_paths,
            &mut report,
            self.validators,
        )?;
        report.replace_final_value(final_value);

        Ok(LoadedConfig { config, report })
    }

    fn pre_deserialize_suggestion_paths(&self, defaults_value: &Value) -> BTreeMap<String, String> {
        let default_known_paths = collect_known_paths_from_value(defaults_value);
        collect_suggestion_paths(&self.metadata, &default_known_paths)
    }

    fn apply_migrations(
        &self,
        merged: &mut Value,
        report: &mut ConfigReport,
    ) -> Result<(), ConfigError> {
        if let Some((version_path, current_version)) = &self.config_version {
            let version_path = normalize_version_registration_path(version_path)?;
            apply_config_migrations(
                merged,
                &version_path,
                *current_version,
                &self.migrations,
                report,
            )?;
        }

        Ok(())
    }

    fn deserialize_config(
        &self,
        merged: &Value,
        report: &ConfigReport,
        string_coercion_paths: &BTreeSet<String>,
        pre_deserialize_suggestion_paths: &BTreeMap<String, String>,
    ) -> Result<T, ConfigError> {
        match deserialize_with_path(merged, report, string_coercion_paths) {
            Ok(config) => Ok(config),
            Err(error) => {
                if let Some(unknown_error) = pre_deserialize_unknown_fields_error::<T>(
                    self.unknown_field_policy,
                    merged,
                    &self.metadata,
                    pre_deserialize_suggestion_paths,
                    report,
                    string_coercion_paths,
                    &error,
                ) {
                    return Err(unknown_error);
                }
                Err(error)
            }
        }
    }

    fn apply_post_deserialize_unknown_policy(
        &self,
        config: &T,
        merged: &Value,
        report: &mut ConfigReport,
        string_coercion_paths: &BTreeSet<String>,
    ) -> Result<(), ConfigError> {
        let known_paths = collect_known_paths(config)?;
        let suggestion_paths = collect_suggestion_paths(&self.metadata, &known_paths);
        apply_unknown_field_policy::<T>(
            self.unknown_field_policy,
            merged,
            &suggestion_paths,
            report,
            string_coercion_paths,
        )
    }
}
