// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

// See the following for inspiration:
// * https://github.com/evmos/ethermint/blob/ebbe0ffd0d474abd745254dc01e60273ea758dae/rpc/namespaces/ethereum/eth/api.go#L44
// * https://github.com/filecoin-project/lotus/blob/v1.23.1-rc2/api/api_full.go#L783

use ethers_core::types::{self as ethtypes, BlockId};
use fendermint_rpc::client::TendermintClient;
use fendermint_rpc::query::QueryClient;
use fendermint_vm_actor_interface::eam::EthAddress;
use fendermint_vm_core::chainid;
use fvm_shared::{address::Address, error::ExitCode};
use jsonrpc_v2::{ErrorLike, Params};
use tendermint_rpc::{endpoint::block, Client};

use crate::{tm, JsonRpcData, JsonRpcResult};

/// Returns a list of addresses owned by client.
///
/// It will always return [] since we don't expect Fendermint to manage private keys.
pub async fn accounts<C>(_data: JsonRpcData<C>) -> JsonRpcResult<Vec<ethtypes::Address>> {
    Ok(vec![])
}

/// Returns the number of most recent block.
pub async fn block_number<C>(data: JsonRpcData<C>) -> JsonRpcResult<ethtypes::U64>
where
    C: Client + Sync,
{
    let res: block::Response = data.client.underlying().latest_block().await?;
    let height = res.block.header.height;
    Ok(ethtypes::U64::from(height.value()))
}

/// Returns the chain ID used for signing replay-protected transactions.
pub async fn chain_id<C>(data: JsonRpcData<C>) -> JsonRpcResult<ethtypes::U64>
where
    C: Client + Sync,
{
    let genesis: tendermint::Genesis<serde_json::Value> =
        data.client.underlying().genesis().await?;
    let chain_id = chainid::from_str_hashed(genesis.chain_id.as_str())?;
    let chain_id: u64 = chain_id.into();
    Ok(ethtypes::U64::from(chain_id))
}

/// Returns the balance of the account of given address.
///
/// ### Parameters
/// 1. DATA, 20 Bytes - address to check for balance.
/// 2. QUANTITY|TAG - integer block number, or the string "latest", "earliest" or "pending".
pub async fn get_balance<C: Client>(
    data: JsonRpcData<C>,
    Params((addr, block_id)): Params<(ethtypes::Address, ethtypes::BlockId)>,
) -> JsonRpcResult<ethtypes::U256>
where
    C: Client + Sync + Send,
{
    let header = match block_id {
        BlockId::Number(n) => tm::header_by_height(data.client.underlying(), n).await?,
        BlockId::Hash(h) => tm::header_by_hash(data.client.underlying(), h).await?,
    };
    let height = header.height;
    let addr = Address::from(&EthAddress(addr.0));
    let res = data.client.actor_state(&addr, Some(height)).await?;

    match res.value {
        Some((_, state)) => {
            let balance = state.balance.atto();
            let balance = ethtypes::U256::from_big_endian(balance.to_signed_bytes_be().as_ref());
            Ok(balance)
        }
        None => Err(jsonrpc_v2::Error::Full {
            code: ExitCode::USR_NOT_FOUND.code(),
            message: format!("actor {addr} not found"),
            data: None,
        }),
    }
}

/// Returns the number of transactions in a block matching the given block number.
///
/// ### Parameters
/// 1. QUANTITY|TAG - integer of a block number, or the string "earliest", "latest" or "pending", as in the default block parameter.
pub async fn get_block_transaction_count_by_number<C: Client>(
    data: JsonRpcData<C>,
    Params((block_number,)): Params<(ethtypes::BlockNumber,)>,
) -> JsonRpcResult<ethtypes::U64>
where
    C: Client + Sync,
{
    let block = tm::block_by_height(data.client.underlying(), block_number).await?;

    Ok(ethtypes::U64::from(block.data.len()))
}

/// Returns the number of uncles in a block from a block matching the given block hash.
///
/// It will always return 0 since Tendermint doesn't have uncles.
pub async fn get_uncle_count_by_block_hash<C>(
    _data: JsonRpcData<C>,
    _params: Params<(ethtypes::H256,)>,
) -> JsonRpcResult<ethtypes::U256> {
    Ok(ethtypes::U256::zero())
}

/// Returns the number of uncles in a block from a block matching the given block number.
///
/// It will always return 0 since Tendermint doesn't have uncles.
pub async fn get_uncle_count_by_block_number<C>(
    _data: JsonRpcData<C>,
    _params: Params<(ethtypes::BlockNumber,)>,
) -> JsonRpcResult<ethtypes::U256> {
    Ok(ethtypes::U256::zero())
}
