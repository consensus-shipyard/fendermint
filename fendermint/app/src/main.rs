// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use fendermint_abci::ApplicationService;
use fendermint_app::{
    options::{Command, Options},
    settings::Settings,
    App, AppStore,
};
use fendermint_rocksdb::RocksDb;
use fendermint_vm_interpreter::{
    bytes::BytesMessageInterpreter, chain::ChainMessageInterpreter, fvm::FvmMessageInterpreter,
    signed::SignedMessageInterpreter,
};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let opts = Options::parse();

    // Log events to stdout.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(opts.tracing_level())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    match opts.command {
        Some(Command::Run { ref mode }) => {
            let config_dir = match opts.config_dir() {
                Some(d) if d.is_dir() => d,
                Some(d) if d.exists() => panic!("config '{d:?}' is a not a directory"),
                Some(d) => panic!("config '{d:?}' does not exist"),
                None => panic!("could not find a config directory to use"),
            };

            let _settings = Settings::new(config_dir, mode).expect("error parsing settings");

            let interpreter = FvmMessageInterpreter::<RocksDb>::new();
            let interpreter = SignedMessageInterpreter::new(interpreter);
            let interpreter = ChainMessageInterpreter::new(interpreter);
            let interpreter = BytesMessageInterpreter::new(interpreter);

            let db = open_db();
            // TODO: Read the bundle path from config.
            let bundle_path = bundle_path();
            let app_ns = db.new_cf_handle("app").unwrap();
            let state_hist_ns = db.new_cf_handle("state_hist").unwrap();
            let app =
                App::<_, AppStore, _>::new(db, bundle_path, app_ns, state_hist_ns, interpreter);
            let _service = ApplicationService(app);
        }
        None => {}
    }
}

fn open_db() -> RocksDb {
    todo!()
}

// TODO: Read from config instead of env var with a fallback to hardcoded path.
fn bundle_path() -> PathBuf {
    let bundle_path = std::env::var("BUILTIN_ACTORS_BUNDLE")
        .unwrap_or_else(|_| "../../../builtin-actors/output/bundle.car".to_owned());

    PathBuf::from_str(&bundle_path).expect("malformed bundle path")
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
