use std::path::Path;

use serde_json::Value;

use crate::ConfigError;
#[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
use crate::error::LineColumn;

use super::FileFormat;

pub(super) fn infer_format(path: &Path) -> Result<FileFormat, ConfigError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| ConfigError::InvalidArg {
            arg: path.display().to_string(),
            message: "cannot infer file format without an extension".to_owned(),
        })?;

    match extension.as_str() {
        "json" => Ok(FileFormat::Json),
        "toml" => Ok(FileFormat::Toml),
        "yaml" | "yml" => Ok(FileFormat::Yaml),
        other => Err(ConfigError::InvalidArg {
            arg: path.display().to_string(),
            message: format!("unsupported file format extension: {other}"),
        }),
    }
}

pub(super) fn parse_file_value(
    path: &Path,
    content: &str,
    format: FileFormat,
) -> Result<Value, ConfigError> {
    match format {
        FileFormat::Json => parse_json_value(path, content, format),
        FileFormat::Toml => parse_toml_value(path, content, format),
        FileFormat::Yaml => parse_yaml_value(path, content, format),
    }
}

fn parse_json_value(path: &Path, content: &str, format: FileFormat) -> Result<Value, ConfigError> {
    #[cfg(feature = "json")]
    {
        let value = serde_json::from_str(content).map_err(|error| ConfigError::ParseFile {
            path: path.to_path_buf(),
            format,
            location: Some(LineColumn {
                line: error.line(),
                column: error.column(),
            }),
            message: error.to_string(),
        })?;
        Ok(value)
    }

    #[cfg(not(feature = "json"))]
    {
        let _ = (path, content, format);
        Err(ConfigError::InvalidArg {
            arg: "json".to_owned(),
            message: "json support is disabled for this build".to_owned(),
        })
    }
}

fn parse_toml_value(path: &Path, content: &str, format: FileFormat) -> Result<Value, ConfigError> {
    #[cfg(feature = "toml")]
    {
        let value =
            toml::from_str::<toml::Value>(content).map_err(|error| ConfigError::ParseFile {
                path: path.to_path_buf(),
                format,
                location: error
                    .span()
                    .map(|span| offset_to_line_column(content, span.start)),
                message: error.to_string(),
            })?;
        serde_json::to_value(value).map_err(ConfigError::from)
    }

    #[cfg(not(feature = "toml"))]
    {
        let _ = (path, content, format);
        Err(ConfigError::InvalidArg {
            arg: "toml".to_owned(),
            message: "toml support is disabled for this build".to_owned(),
        })
    }
}

fn parse_yaml_value(path: &Path, content: &str, format: FileFormat) -> Result<Value, ConfigError> {
    #[cfg(feature = "yaml")]
    {
        let value =
            serde_yaml::from_str::<Value>(content).map_err(|error| ConfigError::ParseFile {
                path: path.to_path_buf(),
                format,
                location: error.location().map(|location| LineColumn {
                    line: location.line(),
                    column: location.column(),
                }),
                message: error.to_string(),
            })?;
        Ok(value)
    }

    #[cfg(not(feature = "yaml"))]
    {
        let _ = (path, content, format);
        Err(ConfigError::InvalidArg {
            arg: "yaml".to_owned(),
            message: "yaml support is disabled for this build".to_owned(),
        })
    }
}

#[cfg(feature = "toml")]
fn offset_to_line_column(input: &str, offset: usize) -> LineColumn {
    let mut line = 1;
    let mut column = 1;
    for (index, byte) in input.bytes().enumerate() {
        if index == offset {
            break;
        }
        if byte == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    LineColumn { line, column }
}
