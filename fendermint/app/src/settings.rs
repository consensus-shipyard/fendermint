// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Database {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub db: Database,
}

impl Settings {
    /// Load the default configuration from a directory,
    /// then potential overrides specific to the run mode,
    /// then overrides from the local environment.
    pub fn new(config_dir: PathBuf, run_mode: &str) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::from(config_dir.join("default")))
            // Optional mode specific overrides, checked into git.
            .add_source(File::from(config_dir.join(run_mode)).required(false))
            // Optional local overrides, not checked into git.
            .add_source(File::from(config_dir.join("local")).required(false))
            // Add in settings from the environment (with a prefix of FM)
            // e.g. `FM_DB_PATH=./foo/bar ./target/app` would set the database location.
            .add_source(Environment::with_prefix("fm"))
            .build()?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_deserialize()
    }
}
