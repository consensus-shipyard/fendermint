// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! Example of using the RPC library in combination with ethers abigen
//! to programmatically deploy and call a contract.
//!
//! The example assumes that Tendermint and Fendermint have been started
//! and are running locally.
//!
//! # Usage
//! ```text
//! cargo run -p fendermint_rpc --release --example simplecoin -- --secret-key test-network/keys/alice.sk --sequence 0
//! ```

use std::path::PathBuf;

use anyhow::{anyhow, Context};
use clap::Parser;
use ethers::prelude::abigen;
use fendermint_vm_actor_interface::eam::CreateReturn;
use lazy_static::lazy_static;
use tendermint_rpc::Url;
use tracing::Level;

use fvm_ipld_encoding::RawBytes;
use fvm_shared::econ::TokenAmount;

use fendermint_rpc::client::FendermintClient;
use fendermint_rpc::message::{GasParams, MessageFactory};
use fendermint_rpc::tx::{TxClient, TxCommit};

const CONTRACT_HEX: &'static str =
    include_str!("../../../../builtin-actors/actors/evm/tests/contracts/SimpleCoin.bin");

lazy_static! {
    /// Default gas params based on the testkit.
    static ref GAS_PARAMS: GasParams = GasParams {
        gas_limit: 10_000_000_000,
        gas_fee_cap: TokenAmount::default(),
        gas_premium: TokenAmount::default(),
    };
}

// Generate a statically typed interface for the contract.
// This assumes the `builtin-actors` repo is checked in next to Fendermint,
// which the `make actor-bundle` command takes care of if it wasn't.
abigen!(
    SimpleCoin,
    "../../../builtin-actors/actors/evm/tests/contracts/SimpleCoin.abi"
);

// Alternatively we can generate the ABI code as follows:
// ```
//     ethers::prelude::Abigen::new("SimpleCoin", <path-to-abi>)
//         .unwrap()
//         .generate()
//         .unwrap()
//         .write_to_file("./tests/storage_footprint_abi.rs")
//         .unwrap();
// ```
// This approach combined with `build.rs` was explored in https://github.com/filecoin-project/ref-fvm/pull/1507

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

    /// Enable DEBUG logs.
    #[arg(long, short)]
    pub verbose: bool,

    /// Path to the secret key to deploy with, expected to be in Base64 format.
    #[arg(long, short)]
    pub secret_key: PathBuf,

    /// Next nonce of the account.
    #[arg(long, short = 'n', default_value = "0")]
    pub sequence: u64,
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

/// See the module docs for how to run.
#[tokio::main]
async fn main() {
    let opts: Options = Options::parse();

    tracing_subscriber::fmt()
        .with_max_level(opts.log_level())
        .init();

    let mf = MessageFactory::from_file(&opts.secret_key, opts.sequence)
        .expect("error creating message factor");
    let client = FendermintClient::new_http(opts.url, None).expect("error creating client");
    let client = client.bind(mf);
    run(client).await.unwrap();
}

async fn run(mut client: impl TxClient<TxCommit> + Send + Sync) -> anyhow::Result<()> {
    let create_return = deploy(&mut client).await?;

    tracing::debug!(
        create_return = format!("{create_return:?}"),
        "contract deployed"
    );

    Ok(())
}

async fn deploy(
    client: &mut (impl TxClient<TxCommit> + Send + Sync),
) -> anyhow::Result<CreateReturn> {
    let contract = hex::decode(&CONTRACT_HEX).context("error parsing contract")?;

    let res = client
        .fevm_create(
            RawBytes::from(contract),
            RawBytes::default(),
            TokenAmount::default(),
            GAS_PARAMS.clone(),
        )
        .await
        .context("error deploying contract")?;

    let ret = res.return_data.ok_or(anyhow!("no CreateReturn data"))?;

    Ok(ret)
}
