use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::Value;

use crate::error::ValidationErrors;
use crate::patch::DeferredPatchLayer;
use crate::{ConfigMetadata, EnvDecoder};

mod args;
mod builder;
mod canonical;
mod de;
mod env;
mod env_decoder;
mod file;
mod indexed_array;
mod layer;
mod load;
mod loaded;
mod merge;
mod migration;
mod migration_runtime;
mod overrides;
mod path;
mod policy;
#[cfg(feature = "schema")]
mod schema_secrets;
mod secret_path;
mod source;
mod trace;
mod unknown;
mod validation;

pub use self::args::ArgsSource;
pub use self::env::EnvSource;
pub use self::file::{FileFormat, FileSource};
pub use self::layer::Layer;
use self::layer::PendingCustomLayer;
pub use self::loaded::LoadedConfig;
pub use self::migration::{
    ConfigMigration, ConfigMigrationKind, MigrationConflictPolicy, UnknownFieldPolicy,
};
pub(crate) use self::path::record_direct_array_state;
use self::secret_path::SecretPathSpec;
pub use self::source::{SourceKind, SourceTrace};
pub(crate) use self::trace::is_secret_path;

type Normalizer<T> = Box<dyn Fn(&mut T) -> Result<(), String> + Send + Sync>;
type Validator<T> = Box<dyn Fn(&T) -> Result<(), ValidationErrors> + Send + Sync>;
type CustomEnvDecoder = Arc<dyn Fn(&str) -> Result<Value, String> + Send + Sync>;

struct NamedNormalizer<T> {
    name: String,
    run: Normalizer<T>,
}

struct NamedValidator<T> {
    name: String,
    run: Validator<T>,
}

/// Builder for layered configuration loading.
///
/// `ConfigLoader<T>` is the main entry point for `tier`. It starts from
/// in-code defaults and then applies additional layers in a deterministic
/// order. The loader can also attach metadata, secret paths, normalizers, and
/// validators before producing a typed [`LoadedConfig`].
///
/// # Examples
///
/// ```no_run
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigLoader, EnvSource};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     host: String,
///     port: u16,
/// }
///
/// impl Default for AppConfig {
///     fn default() -> Self {
///         Self {
///             host: "127.0.0.1".to_owned(),
///             port: 3000,
///         }
///     }
/// }
///
/// let loaded = ConfigLoader::new(AppConfig::default())
///     .file("config/app.toml")
///     .env(EnvSource::prefixed("APP"))
///     .load()?;
///
/// assert!(loaded.port >= 1);
/// # Ok::<(), tier::ConfigError>(())
/// ```
pub struct ConfigLoader<T> {
    defaults: T,
    files: Vec<FileSource>,
    env_sources: Vec<EnvSource>,
    args_source: Option<ArgsSource>,
    custom_layers: Vec<PendingCustomLayer>,
    typed_arg_layers: Vec<DeferredPatchLayer>,
    metadata: ConfigMetadata,
    secret_paths: BTreeSet<SecretPathSpec>,
    normalizers: Vec<NamedNormalizer<T>>,
    validators: Vec<NamedValidator<T>>,
    profile: Option<String>,
    unknown_field_policy: UnknownFieldPolicy,
    env_decoders: BTreeMap<String, EnvDecoder>,
    custom_env_decoders: BTreeMap<String, CustomEnvDecoder>,
    config_version: Option<(String, u32)>,
    migrations: Vec<ConfigMigration>,
}
