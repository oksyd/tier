use std::ffi::OsString;

use crate::error::ConfigError;

use super::file::FileSource;
use super::overrides::ParsedOverride;
use super::overrides::parse_override_value;
use super::path::try_normalize_external_path_with_explicit_arrays;

mod claim;
pub(super) mod layer;

#[derive(Debug, Clone)]
/// CLI override source definition.
///
/// `ArgsSource` parses the same `--config`, `--profile`, and `--set key=value`
/// flags that `tier` accepts through its reusable `clap` integration.
///
/// # Examples
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use tier::{ArgsSource, ConfigLoader};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// impl Default for AppConfig {
///     fn default() -> Self {
///         Self { port: 3000 }
///     }
/// }
///
/// let loaded = ConfigLoader::new(AppConfig::default())
///     .args(ArgsSource::from_args(["app", "--set", "port=7000"]))
///     .load()?;
///
/// assert_eq!(loaded.port, 7000);
/// # Ok::<(), tier::ConfigError>(())
/// ```
pub struct ArgsSource {
    args: ArgsInput,
}

#[derive(Debug, Clone)]
enum ArgsInput {
    Process,
    Explicit(Vec<OsString>),
}

impl ArgsSource {
    /// Creates a source that reads the current process arguments during loading.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            args: ArgsInput::Process,
        }
    }

    /// Creates an argument source from explicit argv values.
    #[must_use]
    pub fn from_args<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            args: ArgsInput::Explicit(
                iter.into_iter()
                    .map(|arg| OsString::from(arg.into()))
                    .collect(),
            ),
        }
    }

    /// Creates an argument source from potentially non-Unicode platform arguments.
    ///
    /// Encoding errors are returned by [`ConfigLoader::load`](super::ConfigLoader::load)
    /// rather than causing a process-wide panic.
    #[must_use]
    pub fn from_os_args<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        Self {
            args: ArgsInput::Explicit(iter.into_iter().map(Into::into).collect()),
        }
    }
}

pub(super) struct ParsedArgs {
    pub(super) profile: Option<String>,
    pub(super) files: Vec<FileSource>,
    pub(super) overrides: Vec<ParsedArgOverride>,
}

pub(super) struct ParsedArgOverride {
    pub(super) source_name: String,
    pub(super) path: String,
    pub(super) explicit_array_segments: std::collections::BTreeSet<usize>,
    pub(super) parsed: ParsedOverride,
    pub(super) error_arg: String,
}

pub(super) fn parse_args(source: ArgsSource) -> Result<ParsedArgs, ConfigError> {
    let args = match source.args {
        ArgsInput::Process => std::env::args_os().collect(),
        ArgsInput::Explicit(args) => args,
    };
    let args = args
        .into_iter()
        .enumerate()
        .map(|(index, arg)| {
            arg.into_string()
                .map_err(|_| ConfigError::NonUnicodeArgument { index })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut args = args.into_iter();
    let mut files = Vec::new();
    let mut profile = None;
    let mut overrides = Vec::new();

    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--config=") {
            files.push(FileSource::new(value));
            continue;
        }

        if arg == "--config" {
            let value = args.next().ok_or_else(|| ConfigError::MissingArgValue {
                flag: "--config".to_owned(),
            })?;
            files.push(FileSource::new(value));
            continue;
        }

        if let Some(value) = arg.strip_prefix("--profile=") {
            profile = Some(value.to_owned());
            continue;
        }

        if arg == "--profile" {
            profile = Some(args.next().ok_or_else(|| ConfigError::MissingArgValue {
                flag: "--profile".to_owned(),
            })?);
            continue;
        }

        let set_value = if let Some(value) = arg.strip_prefix("--set=") {
            Some(value.to_owned())
        } else if arg == "--set" {
            Some(args.next().ok_or_else(|| ConfigError::MissingArgValue {
                flag: "--set".to_owned(),
            })?)
        } else {
            None
        };

        let Some(set_value) = set_value else {
            continue;
        };

        let (raw_path, raw_value) =
            set_value
                .split_once('=')
                .ok_or_else(|| ConfigError::InvalidArg {
                    arg: "--set".to_owned(),
                    message: "expected key=value".to_owned(),
                })?;
        let (path, explicit_array_segments) =
            try_normalize_external_path_with_explicit_arrays(raw_path).map_err(|message| {
                ConfigError::InvalidArg {
                    arg: format!("--set {raw_path}"),
                    message,
                }
            })?;
        if path.is_empty() {
            return Err(ConfigError::InvalidArg {
                arg: "--set".to_owned(),
                message: "configuration path cannot be empty".to_owned(),
            });
        }

        let parsed =
            parse_override_value(raw_value).map_err(|message| ConfigError::InvalidArg {
                arg: format!("--set {path}"),
                message,
            })?;
        // Provenance and diagnostics are routinely rendered or serialized. Never
        // retain the override value here because the target path may be secret.
        let source_name = format!("--set {raw_path}");
        overrides.push(ParsedArgOverride {
            source_name,
            path: path.clone(),
            explicit_array_segments,
            parsed,
            error_arg: format!("--set {path}"),
        });
    }

    Ok(ParsedArgs {
        profile,
        files,
        overrides,
    })
}
