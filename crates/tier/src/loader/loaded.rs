use crate::report::ConfigReport;

#[cfg(feature = "schema")]
use serde::Serialize;
#[cfg(feature = "schema")]
use serde::de::DeserializeOwned;
#[cfg(feature = "schema")]
use serde_json::Value;

#[cfg(feature = "schema")]
use crate::export::{json_pretty, json_value};

#[derive(Debug)]
/// Loaded configuration plus its diagnostic report.
pub struct LoadedConfig<T> {
    pub(super) config: T,
    pub(super) report: ConfigReport,
}

impl<T> LoadedConfig<T> {
    /// Returns the loaded configuration value.
    #[must_use]
    pub fn config(&self) -> &T {
        &self.config
    }

    /// Returns the diagnostic report associated with the load.
    #[must_use]
    pub fn report(&self) -> &ConfigReport {
        &self.report
    }

    /// Splits the loaded configuration into its value and report.
    pub fn into_parts(self) -> (T, ConfigReport) {
        (self.config, self.report)
    }

    /// Returns the loaded configuration value, discarding the report.
    pub fn into_inner(self) -> T {
        self.config
    }
}

impl<T> Clone for LoadedConfig<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            report: self.report.clone(),
        }
    }
}

impl<T> std::ops::Deref for LoadedConfig<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

#[cfg(feature = "schema")]
impl<T> LoadedConfig<T>
where
    T: Serialize + DeserializeOwned + crate::JsonSchema + crate::TierMetadata,
{
    /// Builds a versioned machine-readable export bundle for downstream tools.
    #[must_use]
    pub fn export_bundle(&self, options: &crate::EnvDocOptions) -> crate::ExportBundleReport {
        crate::ExportBundleReport {
            format_version: crate::EXPORT_BUNDLE_FORMAT_VERSION,
            doctor: self.report.doctor_report(),
            audit: self.report.audit_report(),
            env_docs: crate::env_docs_report::<T>(options),
            json_schema: crate::json_schema_report::<T>(),
            annotated_json_schema: crate::annotated_json_schema_report::<T>(),
            example: crate::config_example_report::<T>(),
        }
    }

    /// Renders the versioned export bundle as JSON.
    #[must_use]
    pub fn export_bundle_json(&self, options: &crate::EnvDocOptions) -> Value {
        json_value(
            &self.export_bundle(options),
            Value::Object(Default::default()),
        )
    }

    /// Renders the versioned export bundle as pretty JSON.
    #[must_use]
    pub fn export_bundle_json_pretty(&self, options: &crate::EnvDocOptions) -> String {
        json_pretty(
            &self.export_bundle_json(options),
            "{\"error\":\"failed to render export bundle\"}",
        )
    }
}
