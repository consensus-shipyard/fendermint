// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use cid::Cid;
use clap::{Args, Subcommand};
use fvm_shared::address::Address;
use tendermint_rpc::Url;

use super::parse::*;

#[derive(Args, Debug)]
pub struct RpcArgs {
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

    #[command(subcommand)]
    pub command: RpcCommands,
}

#[derive(Subcommand, Debug)]
pub enum RpcCommands {
    /// Get raw IPLD content; print it as base64 string.
    Ipld {
        /// Initial balance in atto.
        #[arg(long, short, value_parser = parse_cid)]
        cid: Cid,
    },
    /// Get the state of an actor; print it as JSON.
    ActorState {
        /// Address of the actor to query.
        #[arg(long, short, value_parser = parse_address)]
        address: Address,
    },
}
