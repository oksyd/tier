use serde_json::Value;

/// Stable version tag for machine-readable schema and example export payloads.
pub const SCHEMA_EXPORT_FORMAT_VERSION: u32 = 3;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Versioned machine-readable JSON Schema payload.
pub struct JsonSchemaReport {
    /// Stable schema version for external consumers.
    pub format_version: u32,
    /// Exported JSON Schema document.
    pub schema: Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Versioned machine-readable example configuration payload.
pub struct ConfigExampleReport {
    /// Stable schema version for external consumers.
    pub format_version: u32,
    /// Generated example configuration value.
    pub example: Value,
}
