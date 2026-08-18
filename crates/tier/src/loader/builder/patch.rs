use crate::{ConfigError, TierPatch};

use super::super::{ConfigLoader, PendingCustomLayer, SourceKind, SourceTrace};

impl<T> ConfigLoader<T> {
    /// Adds a typed sparse patch as a custom layer.
    ///
    /// This keeps sparse overrides typed and avoids maintaining a parallel
    /// serializable shadow hierarchy just to build a [`Layer`](crate::Layer).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "derive")] {
    /// # fn main() -> Result<(), tier::ConfigError> {
    /// use serde::{Deserialize, Serialize};
    /// use tier::{ConfigLoader, TierPatch};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct AppConfig {
    ///     port: u16,
    /// }
    ///
    /// #[derive(Debug, TierPatch, Default)]
    /// struct CliPatch {
    ///     port: Option<u16>,
    /// }
    ///
    /// let loaded = ConfigLoader::new(AppConfig { port: 3000 })
    ///     .patch("typed-cli", &CliPatch { port: Some(7000) })?
    ///     .load()?;
    ///
    /// assert_eq!(loaded.port, 7000);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn patch<P>(mut self, name: impl Into<String>, patch: &P) -> Result<Self, ConfigError>
    where
        P: TierPatch,
    {
        let mut builder = crate::patch::PatchLayerBuilder::from_trace_deferred(SourceTrace {
            kind: SourceKind::Custom,
            name: name.into(),
            location: None,
        });
        patch.write_layer(&mut builder, "")?;
        let layer = builder.finish_deferred();
        if !layer.is_empty() {
            self.custom_layers
                .push(PendingCustomLayer::DeferredPatch(layer));
        }
        Ok(self)
    }

    #[cfg(feature = "clap")]
    /// Adds a typed `clap`-style sparse override struct as the last CLI layer.
    ///
    /// This is the ergonomic bridge for applications that already parse a
    /// typed `clap` CLI and want to feed the parsed values into `tier` without
    /// building a manual shadow patch struct.
    ///
    /// `clap` remains responsible for CLI grammar, subcommands, trailing args,
    /// and parse-time validation. `tier` only applies the already-parsed typed
    /// values as a last-layer configuration patch.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(all(feature = "derive", feature = "clap"))] {
    /// # fn main() -> Result<(), tier::ConfigError> {
    /// use clap::Parser;
    /// use serde::{Deserialize, Serialize};
    /// use tier::{ConfigLoader, TierPatch};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct AppConfig {
    ///     port: u16,
    ///     token: Option<String>,
    /// }
    ///
    /// #[derive(Debug, Parser, TierPatch)]
    /// struct AppCli {
    ///     #[arg(long)]
    ///     port: Option<u16>,
    ///     #[arg(long = "db-token")]
    ///     #[tier(path_expr = tier::path!(AppConfig.token))]
    ///     token: Option<String>,
    /// }
    ///
    /// let cli = AppCli::parse_from(["app", "--port", "8080", "--db-token", "from-cli"]);
    /// let loaded = ConfigLoader::new(AppConfig {
    ///         port: 3000,
    ///         token: None,
    ///     })
    ///     .clap_overrides(&cli)?
    ///     .load()?;
    ///
    /// assert_eq!(loaded.port, 8080);
    /// assert_eq!(loaded.token.as_deref(), Some("from-cli"));
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn clap_overrides<P>(mut self, patch: &P) -> Result<Self, ConfigError>
    where
        P: TierPatch,
    {
        let mut builder = crate::patch::PatchLayerBuilder::from_trace_deferred(SourceTrace {
            kind: SourceKind::Arguments,
            name: "typed-clap".to_owned(),
            location: None,
        });
        patch.write_layer(&mut builder, "")?;
        let layer = builder.finish_deferred();
        if !layer.is_empty() {
            self.typed_arg_layers.push(layer);
        }
        Ok(self)
    }

    #[cfg(feature = "clap")]
    /// Projects a parsed CLI value onto the config-bearing patch portion and
    /// applies it as the last CLI layer.
    ///
    /// This is the CLI-first companion to [`ConfigLoader::clap_overrides`].
    /// It lets an application keep a full `clap` model with subcommands,
    /// positional arguments, or trailing args, while only the selected
    /// config-bearing sub-structure participates in `tier` overrides.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(all(feature = "derive", feature = "clap"))] {
    /// # fn main() -> Result<(), tier::ConfigError> {
    /// use clap::{Parser, Subcommand};
    /// use serde::{Deserialize, Serialize};
    /// use tier::{ConfigLoader, TierPatch};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct AppConfig {
    ///     port: u16,
    /// }
    ///
    /// #[derive(Debug, Clone, clap::Args, TierPatch, Default)]
    /// struct ConfigArgs {
    ///     #[arg(long)]
    ///     port: Option<u16>,
    /// }
    ///
    /// #[derive(Debug, Clone, Subcommand)]
    /// enum Command {
    ///     Serve {
    ///         #[arg(last = true)]
    ///         trailing: Vec<String>,
    ///     },
    /// }
    ///
    /// #[derive(Debug, Clone, Parser)]
    /// struct AppCli {
    ///     #[command(flatten)]
    ///     config: ConfigArgs,
    ///     #[command(subcommand)]
    ///     command: Option<Command>,
    /// }
    ///
    /// let cli = AppCli::parse_from(["app", "--port", "8080", "serve", "--", "extra"]);
    /// let loaded = ConfigLoader::new(AppConfig { port: 3000 })
    ///     .clap_overrides_from(&cli, |cli| &cli.config)?
    ///     .load()?;
    ///
    /// assert_eq!(loaded.port, 8080);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn clap_overrides_from<C, P, F>(self, cli: &C, project: F) -> Result<Self, ConfigError>
    where
        P: TierPatch,
        F: FnOnce(&C) -> &P,
    {
        self.clap_overrides(project(cli))
    }
}
