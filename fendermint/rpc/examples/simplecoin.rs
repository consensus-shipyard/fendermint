// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use clap::Parser;
use fendermint_rpc::client::{FendermintClient, TendermintClient};
use fendermint_rpc::tx::{TxClient, TxCommit};
use tendermint_rpc::Url;
use tracing::Level;

#[derive(Parser, Debug)]
pub struct Options {
    /// The URL of the Tendermint node's RPC endpoint.
    #[arg(
        long,
        short,
        default_value = "http://127.0.0.1:26657",
        env = "TENDERMINT_RPC_URL"
    )]
    pub url: Url,

    #[arg(long, short)]
    pub verbose: bool,
}

impl Options {
    pub fn log_level(&self) -> Level {
        if self.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        }
    }
}

#[tokio::main]
async fn main() {
    let opts: Options = Options::parse();

    tracing_subscriber::fmt()
        .with_max_level(opts.log_level())
        .init();

    let client = FendermintClient::new_http(opts.url, None).unwrap();
    let client = client.bind(todo!());
    run(client).await;
}

async fn run(mut client: impl TxClient<TxCommit> + Send + Sync) {
    client
        .fevm_create(todo!(), todo!(), todo!(), todo!())
        .await
        .expect("error deploying contract");
}
