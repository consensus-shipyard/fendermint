// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use self::{eth::EthArgs, genesis::GenesisArgs, key::KeyArgs, rpc::RpcArgs, run::RunArgs};

pub mod eth;
pub mod genesis;
pub mod key;
pub mod rpc;
pub mod run;

mod parse;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Parser, Debug)]
#[command(version)]
pub struct Options {
    /// Set a custom directory for data and configuration files.
    #[arg(
        short = 'd',
        long,
        default_value = "~/.fendermint",
        env = "FM_HOME_DIR"
    )]
    pub home_dir: PathBuf,

    /// Optionally override the default configuration.
    #[arg(short, long, default_value = "dev")]
    pub mode: String,

    /// Set the logging level.
    #[arg(short, long, default_value = "info", value_enum, env = "LOG_LEVEL")]
    pub log_level: LogLevel,

    #[command(subcommand)]
    pub command: Commands,
}

impl Options {
    /// Tracing level, unless it's turned off.
    pub fn tracing_level(&self) -> Option<tracing::Level> {
        match self.log_level {
            LogLevel::Off => None,
            LogLevel::Error => Some(tracing::Level::ERROR),
            LogLevel::Warn => Some(tracing::Level::WARN),
            LogLevel::Info => Some(tracing::Level::INFO),
            LogLevel::Debug => Some(tracing::Level::DEBUG),
            LogLevel::Trace => Some(tracing::Level::TRACE),
        }
    }

    pub fn config_dir(&self) -> PathBuf {
        self.home_dir.join("config")
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the [`App`], listening to ABCI requests from Tendermint.
    Run(RunArgs),
    /// Subcommands related to the construction of signing keys.
    Key(KeyArgs),
    /// Subcommands related to the construction of Genesis files.
    Genesis(GenesisArgs),
    /// Subcommands related to sending JSON-RPC commands/queries to Tendermint.
    Rpc(RpcArgs),
    /// Subcommands related to the Ethereum API facade.
    Eth(EthArgs),
}
