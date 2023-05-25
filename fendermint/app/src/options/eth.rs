// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use clap::{Args, Subcommand};
use tendermint_rpc::Url;

#[derive(Args, Debug)]
pub struct EthArgs {
    #[command(subcommand)]
    pub command: EthCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EthCommands {
    /// Run the Ethereum JSON-RPC facade.
    Run {
        /// The URL of the Tendermint node's RPC endpoint.
        #[arg(
            long,
            short,
            default_value = "http://127.0.0.1:26657",
            env = "TENDERMINT_RPC_URL"
        )]
        url: Url,

        /// An optional HTTP/S proxy through which to submit requests to the
        /// Tendermint node's RPC endpoint.
        #[arg(long)]
        proxy_url: Option<Url>,
    },
}
