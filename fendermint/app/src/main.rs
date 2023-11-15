// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{anyhow, Context};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use trace4rs::config::{AppenderId, Policy, Target};
use trace4rs::{
    config::{self, Config, Format},
    Handle,
};

pub use fendermint_app_options as options;
pub use fendermint_app_settings as settings;
use fendermint_app_settings::expand_tilde;

mod cmd;

#[tokio::main]
async fn main() {
    let opts = options::parse();

    // Log events to stdout.
    if let Some(level) = opts.tracing_level() {
        create_log(level, &opts.log_dir).expect("cannot create logging");
    }

    if let Err(e) = cmd::exec(&opts).await {
        tracing::error!("failed to execute {:?}: {e:?}", opts);
        std::process::exit(1);
    }
}

fn create_log(level: tracing::Level, log_dir: &Path) -> anyhow::Result<()> {
    let log_folder = expand_tilde(log_dir)
        .to_str()
        .ok_or_else(|| anyhow!("cannot parse log folder"))?
        .to_string();
    std::fs::create_dir_all(&log_folder).context("cannot create log folder")?;

    let console_appender = config::Appender::Console;
    let topdown_appender = config::Appender::RollingFile {
        path: format!("{log_folder}/topdown.log"),
        policy: Policy {
            maximum_file_size: "10mb".to_string(),
            // we keep the last 5 log files, older files will be deleted
            max_size_roll_backups: 5,
            pattern: None,
        },
    };

    // debug level logger
    let topdown_logger = config::Logger {
        level: config::LevelFilter::DEBUG,
        appenders: literally::hset! { AppenderId::from("topdown") },
        format: Format::default(),
    };

    // default logging output info log to console
    let default = config::Logger {
        level: config::LevelFilter::from_str(level.as_str())?,
        appenders: literally::hset! { AppenderId::from("console") },
        format: Format::default(),
    };

    let config = Config {
        default,
        loggers: literally::hmap! {
            Target::from("fendermint_vm_topdown") => topdown_logger.clone(),
            Target::from("fendermint_vm_interpreter::chain") => topdown_logger,
        },
        appenders: literally::hmap! {
            AppenderId::from("console") => console_appender,
            AppenderId::from("topdown") => topdown_appender
        },
    };

    let handle = Arc::new(Handle::try_from(config).unwrap());

    tracing::subscriber::set_global_default(handle.subscriber())
        .context("setting default subscriber failed")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use cid::Cid;
    use fendermint_rocksdb::{RocksDb, RocksDbConfig};
    use fendermint_vm_interpreter::fvm::bundle::bundle_path;
    use fvm::machine::Manifest;
    use fvm_ipld_car::load_car_unchecked;
    use fvm_ipld_encoding::CborStore;

    #[tokio::test]
    async fn load_car() {
        // Just to see if dependencies compile together, see if we can load an actor bundle into a temporary RocksDB.
        // Run it with `cargo run -p fendermint_app`

        // Not loading the actors from the library any more. It would be possible, as long as dependencies are aligned.
        // let bundle_car = actors_v10::BUNDLE_CAR;

        let bundle_path = bundle_path();
        let bundle_car = std::fs::read(&bundle_path)
            .unwrap_or_else(|_| panic!("failed to load bundle CAR from {bundle_path:?}"));

        let dir = tempfile::Builder::new()
            .tempdir()
            .expect("error creating temporary path for db");
        let path = dir.path().join("rocksdb");

        let open_db = || {
            RocksDb::open(path.clone(), &RocksDbConfig::default()).expect("error creating RocksDB")
        };
        let db = open_db();

        let cids = load_car_unchecked(&db, bundle_car.as_slice())
            .await
            .expect("error loading bundle CAR");

        let bundle_root = cids.first().expect("there should be 1 CID");

        // Close and reopen.
        drop(db);
        let db = open_db();

        let (manifest_version, manifest_data_cid): (u32, Cid) = db
            .get_cbor(bundle_root)
            .expect("error getting bundle root")
            .expect("bundle root was not in store");

        Manifest::load(&db, &manifest_data_cid, manifest_version).expect("error loading manifest");
    }
}
