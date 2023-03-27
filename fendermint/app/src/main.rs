// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use clap::Parser;
use tracing_subscriber::FmtSubscriber;

mod cmd;
mod options;
mod settings;

use options::{Commands, Options};

#[tokio::main]
async fn main() {
    let opts = Options::parse();

    // Log events to stdout.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(opts.tracing_level())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    if let Some(ref cmd) = opts.command {
        let result = match cmd {
            Commands::Run { mode } => cmd::run::run(opts.config_dir(), mode.as_ref()).await,
        };
        if let Err(e) = result {
            tracing::error!("failed to execute {cmd:?}: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use fendermint_rocksdb::{RocksDb, RocksDbConfig};
    use fendermint_vm_interpreter::fvm::bundle::bundle_path;
    use fvm_ipld_car::load_car_unchecked;

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
        let db =
            RocksDb::open(path.clone(), &RocksDbConfig::default()).expect("error creating RocksDB");

        let _cids = load_car_unchecked(&db, bundle_car.as_slice())
            .await
            .expect("error loading bundle CAR");
    }
}
