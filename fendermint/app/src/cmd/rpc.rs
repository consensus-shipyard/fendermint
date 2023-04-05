// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::PathBuf;

use anyhow::Context;
use base64::Engine;
use bytes::Bytes;
use fendermint_vm_message::chain::ChainMessage;
use fvm_ipld_encoding::{BytesDe, RawBytes};
use fvm_shared::address::Address;
use fvm_shared::econ::TokenAmount;
use fvm_shared::MethodNum;
use serde::Serialize;
use serde_json::json;
use tendermint::block::Height;
use tendermint_rpc::endpoint::broadcast;
use tendermint_rpc::v0_37::Client;

use fendermint_rpc::message::{GasParams, MessageFactory};
use fendermint_rpc::{client::FendermintClient, query::QueryClient};
use fendermint_vm_actor_interface::eam::{self, CreateReturn};

use crate::cmd;
use crate::options::rpc::{BroadcastMode, RpcFevmCommands, TransArgs};
use crate::{
    cmd::to_b64,
    options::rpc::{RpcArgs, RpcCommands, RpcQueryCommands},
};
use anyhow::anyhow;

use super::key::read_secret_key;

cmd! {
  RpcArgs(self) {
    let client = FendermintClient::new(self.url.clone(), self.proxy_url.clone())?;
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
        RpcFevmCommands::Invoke { contract, method, method_args } => {
            fevm_invoke(client, args, contract, method, method_args).await
        }
      }
    }
  }
}

/// Run an ABCI query and print the results on STDOUT.
async fn query(
    client: FendermintClient,
    height: Height,
    command: &RpcQueryCommands,
) -> anyhow::Result<()> {
    match command {
        RpcQueryCommands::Ipld { cid } => match client.ipld(cid).await? {
            Some(data) => println!("{}", to_b64(&data)),
            None => eprintln!("CID not found"),
        },
        RpcQueryCommands::ActorState { address } => {
            match client.actor_state(address, height).await? {
                Some((id, state)) => {
                    let out = json! ({
                      "id": id,
                      "state": state,
                    });

                    // Print JSON as a single line - we can display it nicer with `jq` if needed.
                    let json = serde_json::to_string(&out)?;

                    println!("{}", json)
                }
                None => {
                    eprintln!("actor not found")
                }
            }
        }
    };
    Ok(())
}

/// Execute token transfer through RPC and print the response to STDOUT as JSON.
async fn transfer(client: FendermintClient, args: &TransArgs, to: &Address) -> anyhow::Result<()> {
    let data = message_payload(args, |mf, v, g| mf.transfer(*to, v, g))?;
    broadcast_and_print(client, data, args.broadcast_mode, |_| None).await
}

/// Execute a transaction through RPC and print the response to STDOUT as JSON.
async fn transact(
    client: FendermintClient,
    args: &TransArgs,
    to: &Address,
    method_num: MethodNum,
    params: RawBytes,
) -> anyhow::Result<()> {
    let data = message_payload(args, |mf, v, g| {
        mf.transaction(*to, method_num, params, v, g)
    })?;
    broadcast_and_print(client, data, args.broadcast_mode, |_| None).await
}

/// Deploy an EVM contract through RPC and print the response to STDOUT as JSON.
async fn fevm_create(
    client: FendermintClient,
    args: &TransArgs,
    contract: &PathBuf,
    constructor_args: &RawBytes,
) -> anyhow::Result<()> {
    let contract_hex = std::fs::read_to_string(contract).context("failed to read contract")?;
    let contract_bytes = hex::decode(contract_hex).context("failed to parse contract from hex")?;
    let contract_bytes = RawBytes::from(contract_bytes);

    let data = message_payload(args, |mf, v, g| {
        mf.fevm_create(contract_bytes, constructor_args.clone(), v, g)
    })?;

    broadcast_and_print(client, data, args.broadcast_mode, |data| {
        Some(
            parse_data(data)
                .and_then(parse_create_return)
                .map(create_return_to_json),
        )
    })
    .await
}

/// Deploy an EVM contract through RPC and print the response to STDOUT as JSON.
async fn fevm_invoke(
    client: FendermintClient,
    args: &TransArgs,
    contract: &Address,
    method: &RawBytes,
    method_args: &RawBytes,
) -> anyhow::Result<()> {
    let data = message_payload(args, |mf, v, g| {
        mf.fevm_invoke(*contract, method.clone(), method_args.clone(), v, g)
    })?;

    broadcast_and_print(client, data, args.broadcast_mode, |data| {
        Some(
            parse_data(data)
                .and_then(|data| {
                    fvm_ipld_encoding::from_slice::<BytesDe>(&data)
                        .map(|bz| bz.0)
                        .map_err(|e| anyhow!("failed to deserialize bytes: {e}"))
                })
                .map(|bz| serde_json::Value::String(hex::encode(bz))),
        )
    })
    .await
}

/// Broadcast a transaction to tendermint and print the results to STDOUT as JSON.
async fn broadcast_and_print<F>(
    client: FendermintClient,
    data: Vec<u8>,
    mode: BroadcastMode,
    parse_data: F,
) -> anyhow::Result<()>
where
    F: FnOnce(&Bytes) -> Option<anyhow::Result<serde_json::Value>>,
{
    match mode {
        BroadcastMode::Async => print_json(
            client.inner().broadcast_tx_async(data).await?,
            |_| None,
            parse_data,
        ),
        BroadcastMode::Sync => print_json(
            client.inner().broadcast_tx_sync(data).await?,
            |_| None,
            parse_data,
        ),
        BroadcastMode::Commit => print_json(
            client.inner().broadcast_tx_commit(data).await?,
            |r: &broadcast::tx_commit::Response| Some(&r.deliver_tx.data),
            parse_data,
        ),
    }
}

/// Display some value as JSON.
fn print_json<T, G, F>(value: T, get_data: G, parse_data: F) -> anyhow::Result<()>
where
    T: Serialize,
    G: FnOnce(&T) -> Option<&Bytes>,
    F: FnOnce(&Bytes) -> Option<anyhow::Result<serde_json::Value>>,
{
    let response = serde_json::to_value(&value)?;
    let output = {
        let return_data = match get_data(&value) {
            None => None,
            Some(bz) if bz.is_empty() => None,
            Some(bz) => match parse_data(bz) {
                None => None,
                Some(Ok(return_json)) => Some(return_json),
                Some(Err(e)) => Some(json!({
                    "error": format!("error parsing return data: {e}")
                })),
            },
        };
        match return_data {
            Some(return_data) => json!({"response": response, "return_data": return_data}),
            None => json!({ "response": response }),
        }
    };
    // Using "jsonline"; use `jq` to format.
    let json = serde_json::to_string(&output)?;
    println!("{}", json);
    Ok(())
}

fn message_payload<F>(args: &TransArgs, f: F) -> anyhow::Result<Vec<u8>>
where
    F: FnOnce(&mut MessageFactory, TokenAmount, GasParams) -> anyhow::Result<ChainMessage>,
{
    let sk = read_secret_key(&args.secret_key)?;
    let mut mf = MessageFactory::new(sk, args.sequence)?;
    let gas_params = GasParams {
        gas_fee_cap: args.gas_fee_cap.clone(),
        gas_limit: args.gas_limit,
        gas_premium: args.gas_premium.clone(),
    };
    let message = f(&mut mf, args.value.clone(), gas_params)?;
    let data = MessageFactory::serialize(&message)?;
    Ok(data)
}

/// Parse what Tendermint returns in the `data` field of [`DeliverTx`] into bytes.
/// It looks like somewhere along the way it replaces them with the bytes of a Base64 encoded string.
fn parse_data(data: &Bytes) -> anyhow::Result<Vec<u8>> {
    let b64 = String::from_utf8(data.to_vec()).context("error parsing data as base64 string")?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .context("error parsing base64 to bytes")?;
    Ok(data)
}

/// Parse what Tendermint returns in the `data` field of `DeliverTx` as `CreateReturn`.
fn parse_create_return(data: Vec<u8>) -> anyhow::Result<CreateReturn> {
    fvm_ipld_encoding::from_slice::<eam::CreateReturn>(&data)
        .map_err(|e| anyhow!("error parsing as CreateReturn: {e}"))
}

fn create_return_to_json(ret: CreateReturn) -> serde_json::Value {
    // Print all the various addresses we can use to refer to an EVM contract.
    // The only reference I can point to about how to use them are the integration tests:
    // https://github.com/filecoin-project/ref-fvm/pull/1507
    // IIRC to call the contract we need to use the `actor_address` or the `delegated_address` in `to`.
    json!({
        "actor_id": ret.actor_id,
        "actor_address": Address::new_id(ret.actor_id).to_string(),
        "actor_id_as_eth_address": hex::encode(eam::EthAddress::from_id(ret.actor_id).0),
        "eth_address": hex::encode(ret.eth_address.0),
        "delegated_address": Address::new_delegated(eam::EAM_ACTOR_ID, &ret.eth_address.0).ok().map(|a| a.to_string()),
        "robust_address": ret.robust_address.map(|a| a.to_string())
    })
}
