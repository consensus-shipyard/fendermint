// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! CLI command implementations.

use crate::{
    options::{Commands, Options},
    settings::Settings,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;

pub mod keygen;
pub mod run;

#[async_trait]
pub trait Cmd {
    type Settings;
    async fn exec(&self, settings: Self::Settings) -> anyhow::Result<()>;
}

/// Convenience macro to simplify declaring commands that either need or don't need settings.
///
/// ```text
/// cmd! {
///   <type-name>(self, settings) {
///     <exec-body>
///   }
/// }
/// ```
#[macro_export]
macro_rules! cmd {
    // A command which needs access to some settings.
    ($name:ident($self:ident, $settings_name:ident : $settings_type:ty) $exec:expr) => {
        #[async_trait::async_trait]
        impl $crate::cmd::Cmd for $name {
            type Settings = $settings_type;

            async fn exec(&$self, $settings_name: Self::Settings) -> anyhow::Result<()> {
                $exec
            }
        }
    };

    // A command which works on the full `Settings`.
    ($name:ident($self:ident, $settings:ident) $exec:expr) => {
        cmd!($name($self, _settings: $crate::settings::Settings) $exec);
    };

    // A command which is self-contained and doesn't need any settings.
    ($name:ident($self:ident) $exec:expr) => {
        cmd!($name($self, _settings: ()) $exec);
    };
}

/// Execute the command specified in the options.
pub async fn exec(opts: &Options) -> anyhow::Result<()> {
    match &opts.command {
        Commands::Run(args) => args.exec(settings(opts)?).await,
        Commands::Keygen(args) => args.exec(()).await,
    }
}

/// Try to parse the settings in the configuration directory.
fn settings(opts: &Options) -> anyhow::Result<Settings> {
    let config_dir = match opts.config_dir() {
        Some(d) if d.is_dir() => d,
        Some(d) if d.exists() => return Err(anyhow!("config '{d:?}' is a not a directory")),
        Some(d) => return Err(anyhow!("config '{d:?}' does not exist")),
        None => return Err(anyhow!("could not find a config directory to use")),
    };

    let settings = Settings::new(config_dir, &opts.mode).context("error parsing settings")?;

    Ok(settings)
}
