// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::PathBuf;

use anyhow::Context;
use fendermint_vm_actor_interface::eam;
use fendermint_vm_interpreter::fvm::{FvmMessage, FvmQuery};
use fendermint_vm_message::chain::ChainMessage;
use fendermint_vm_message::query::ActorState;
use fendermint_vm_message::signed::SignedMessage;
use fvm_ipld_encoding::RawBytes;
use fvm_shared::address::Address;
use fvm_shared::{ActorID, MethodNum, METHOD_SEND};
use libsecp256k1::PublicKey;
use serde::Serialize;
use serde_json::json;
use tendermint::block::Height;
use tendermint_rpc::{endpoint::abci_query::AbciQuery, v0_37::Client, HttpClient, Scheme, Url};

use crate::cmd;
use crate::options::rpc::{BroadcastMode, RpcFevmCommands, TransArgs};
use crate::{
    cmd::to_b64,
    options::rpc::{RpcArgs, RpcCommands, RpcQueryCommands},
};
use anyhow::anyhow;

use super::key::read_secret_key;

// TODO: We should probably make a client interface for the operations we commonly do.

cmd! {
  RpcArgs(self) {
    let client = http_client(self.url.clone(), self.proxy_url.clone())?;
    match &self.command {
      RpcCommands::Query { height, command } => {
        let height = Height::try_from(*height)?;
        query(client, height, command).await
      },
      RpcCommands::Transfer { args, to } => {
        transfer(client, args, to).await
      },
      RpcCommands::Transact { args, to, method_number, params } => {
        transact(client, args, to, *method_number, params.clone()).await
      },
      RpcCommands::Fevm { args, command } => match command {
        RpcFevmCommands::Create { contract, constructor_args } => {
            fevm_create(client, args, contract, constructor_args).await
        }
      }
    }
  }
}

async fn query(
    client: HttpClient,
    height: Height,
    command: &RpcQueryCommands,
) -> anyhow::Result<()> {
    match command {
        RpcQueryCommands::Ipld { cid } => {
            abci_query_print(client, height, FvmQuery::Ipld(*cid), |res| {
                Ok(to_b64(&res.value))
            })
            .await
        }
        RpcQueryCommands::ActorState { address } => {
            abci_query_print(client, height, FvmQuery::ActorState(*address), |res| {
                let state: ActorState =
                    fvm_ipld_encoding::from_slice(&res.value).context("failed to decode state")?;
                let id: ActorID =
                    fvm_ipld_encoding::from_slice(&res.key).context("failed to decode ID")?;

                let out = json! ({
                  "id": id,
                  "state": state,
                });

                // Print JSON as a single line - we can display it nicer with `jq` if needed.
                let json = serde_json::to_string(&out)?;

                Ok(json)
            })
            .await
        }
    }
}

/// Execute token transfer through RPC and print the response to STDOUT as JSON.
async fn transfer(client: HttpClient, args: &TransArgs, to: &Address) -> anyhow::Result<()> {
    let data = transfer_payload(args, to)?;
    broadcast_and_print(client, data, args.broadcast_mode).await
}

/// Execute a transaction through RPC and print the response to STDOUT as JSON.
async fn transact(
    client: HttpClient,
    args: &TransArgs,
    to: &Address,
    method_num: MethodNum,
    params: RawBytes,
) -> anyhow::Result<()> {
    let data = transaction_payload(args, to, method_num, params)?;
    broadcast_and_print(client, data, args.broadcast_mode).await
}

/// Deploy an EVM contract through RPC and print the response to STDOUT as JSON.
async fn fevm_create(
    client: HttpClient,
    args: &TransArgs,
    contract: &PathBuf,
    constructor_args: &RawBytes,
) -> anyhow::Result<()> {
    let contract_hex = std::fs::read_to_string(contract).context("failed to read contract")?;
    let contract_bytes = hex::decode(contract_hex).context("failed to parse contract from hex")?;
    let initcode = [contract_bytes, constructor_args.to_vec()].concat();
    let initcode = RawBytes::serialize(initcode)?;
    let data = transaction_payload(
        args,
        &eam::EAM_ACTOR_ADDR,
        eam::Method::CreateExternal as u64,
        initcode,
    )?;
    broadcast_and_print(client, data, args.broadcast_mode).await
}

/// Broadcast a transaction to tendermint and print the results to STDOUT as JSON.
async fn broadcast_and_print(
    client: HttpClient,
    data: Vec<u8>,
    mode: BroadcastMode,
) -> anyhow::Result<()> {
    match mode {
        BroadcastMode::Async => print_json(client.broadcast_tx_async(data).await?),
        BroadcastMode::Sync => print_json(client.broadcast_tx_async(data).await?),
        BroadcastMode::Commit => print_json(client.broadcast_tx_commit(data).await?),
    }
}

/// Display some value as JSON.
fn print_json<T: Serialize>(value: T) -> anyhow::Result<()> {
    // Using "jsonline"; use `jq` to format.
    let json = serde_json::to_string(&value)?;
    println!("{}", json);
    Ok(())
}

/// Construct transfer payload.
fn transfer_payload(args: &TransArgs, to: &Address) -> anyhow::Result<Vec<u8>> {
    transaction_payload(args, to, METHOD_SEND, Default::default())
}

/// Construct transaction payload.
fn transaction_payload(
    args: &TransArgs,
    to: &Address,
    method_num: MethodNum,
    params: fvm_ipld_encoding::RawBytes,
) -> anyhow::Result<Vec<u8>> {
    let sk = read_secret_key(&args.secret_key)?;
    let pk = PublicKey::from_secret_key(&sk);
    let from = Address::new_secp256k1(&pk.serialize())?;
    let message = FvmMessage {
        version: Default::default(), // TODO: What does this do?
        from,
        to: to.clone(),
        sequence: args.sequence,
        value: args.value.clone(),
        method_num,
        params,
        gas_limit: args.gas_limit,
        gas_fee_cap: args.gas_fee_cap.clone(),
        gas_premium: args.gas_premium.clone(),
    };
    let signed = SignedMessage::new_secp256k1(message, &sk)?;
    let chain = ChainMessage::Signed(Box::new(signed));
    let data = fvm_ipld_encoding::to_vec(&chain)?;
    Ok(data)
}

/// Fetch the query result from the server and print something to STDOUT.
async fn abci_query_print<F>(
    client: HttpClient,
    height: Height,
    query: FvmQuery,
    render: F,
) -> anyhow::Result<()>
where
    F: FnOnce(AbciQuery) -> anyhow::Result<String>,
{
    let data = fvm_ipld_encoding::to_vec(&query).context("failed to encode query")?;
    let res: AbciQuery = client.abci_query(None, data, Some(height), false).await?;
    if res.code.is_ok() {
        let output = render(res)?;
        println!("{}", output);
    } else {
        eprintln!("query returned non-zero exit code: {}", res.code.value());
    }
    Ok(())
}

// Retrieve the proxy URL with precedence:
// 1. If supplied, that's the proxy URL used.
// 2. If not supplied, but environment variable HTTP_PROXY or HTTPS_PROXY are
//    supplied, then use the appropriate variable for the URL in question.
//
// Copied from `tendermint_rpc`.
fn get_http_proxy_url(url_scheme: Scheme, proxy_url: Option<Url>) -> anyhow::Result<Option<Url>> {
    match proxy_url {
        Some(u) => Ok(Some(u)),
        None => match url_scheme {
            Scheme::Http => std::env::var("HTTP_PROXY").ok(),
            Scheme::Https => std::env::var("HTTPS_PROXY")
                .ok()
                .or_else(|| std::env::var("HTTP_PROXY").ok()),
            _ => {
                if std::env::var("HTTP_PROXY").is_ok() || std::env::var("HTTPS_PROXY").is_ok() {
                    tracing::warn!(
                        "Ignoring HTTP proxy environment variables for non-HTTP client connection"
                    );
                }
                None
            }
        }
        .map(|u| u.parse::<Url>().map_err(|e| anyhow!(e)))
        .transpose(),
    }
}

fn http_client(url: Url, proxy_url: Option<Url>) -> anyhow::Result<HttpClient> {
    let proxy_url = get_http_proxy_url(url.scheme(), proxy_url)?;
    let client = match proxy_url {
        Some(proxy_url) => {
            tracing::debug!(
                "Using HTTP client with proxy {} to submit request to {}",
                proxy_url,
                url
            );
            HttpClient::new_with_proxy(url, proxy_url)?
        }
        None => {
            tracing::debug!("Using HTTP client to submit request to: {}", url);
            HttpClient::new(url)?
        }
    };
    Ok(client)
}
