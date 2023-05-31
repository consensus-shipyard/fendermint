// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

// See the following for inspiration:
// * https://github.com/evmos/ethermint/blob/ebbe0ffd0d474abd745254dc01e60273ea758dae/rpc/namespaces/ethereum/eth/api.go#L44
// * https://github.com/filecoin-project/lotus/blob/v1.23.1-rc2/api/api_full.go#L783

use ethers_core::types::{self as et, BlockId};
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
pub async fn accounts<C>(_data: JsonRpcData<C>) -> JsonRpcResult<Vec<et::Address>> {
    Ok(vec![])
}

/// Returns the number of most recent block.
pub async fn block_number<C>(data: JsonRpcData<C>) -> JsonRpcResult<et::U64>
where
    C: Client + Sync,
{
    let res: block::Response = data.client.underlying().latest_block().await?;
    let height = res.block.header.height;
    Ok(et::U64::from(height.value()))
}

/// Returns the chain ID used for signing replay-protected transactions.
pub async fn chain_id<C>(data: JsonRpcData<C>) -> JsonRpcResult<et::U64>
where
    C: Client + Sync,
{
    let genesis: tendermint::Genesis<serde_json::Value> =
        data.client.underlying().genesis().await?;
    let chain_id = chainid::from_str_hashed(genesis.chain_id.as_str())?;
    let chain_id: u64 = chain_id.into();
    Ok(et::U64::from(chain_id))
}

/// Returns the balance of the account of given address.
pub async fn get_balance<C: Client>(
    data: JsonRpcData<C>,
    Params((addr, block_id)): Params<(et::Address, et::BlockId)>,
) -> JsonRpcResult<et::U256>
where
    C: Client + Sync + Send,
{
    let header = match block_id {
        BlockId::Number(n) => tm::header_by_height(data.client.underlying(), n).await?,
        BlockId::Hash(h) => tm::header_by_hash(data.client.underlying(), h).await?,
    };
    let height = header.height;
    let addr = h160_to_fvm_addr(addr);
    let res = data.client.actor_state(&addr, Some(height)).await?;

    match res.value {
        Some((_, state)) => {
            let balance = state.balance.atto();
            let balance = et::U256::from_big_endian(balance.to_signed_bytes_be().as_ref());
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
pub async fn get_block_transaction_count_by_number<C: Client>(
    data: JsonRpcData<C>,
    Params((block_number,)): Params<(et::BlockNumber,)>,
) -> JsonRpcResult<et::U64>
where
    C: Client + Sync,
{
    let block = tm::block_by_height(data.client.underlying(), block_number).await?;

    Ok(et::U64::from(block.data.len()))
}

/// Returns the number of transactions sent from an address, up to a specific block.
pub async fn get_transaction_count<C: Client>(
    data: JsonRpcData<C>,
    Params((addr, block_id)): Params<(et::Address, et::BlockId)>,
) -> JsonRpcResult<et::U64>
where
    C: Client + Sync + Send,
{
    let header = match block_id {
        BlockId::Number(n) => tm::header_by_height(data.client.underlying(), n).await?,
        BlockId::Hash(h) => tm::header_by_hash(data.client.underlying(), h).await?,
    };
    let height = header.height;
    let addr = h160_to_fvm_addr(addr);
    let res = data.client.actor_state(&addr, Some(height)).await?;

    match res.value {
        Some((_, state)) => {
            let nonce = state.sequence;
            Ok(et::U64::from(nonce))
        }
        None => Err(jsonrpc_v2::Error::Full {
            code: ExitCode::USR_NOT_FOUND.code(),
            message: format!("actor {addr} not found"),
            data: None,
        }),
    }
}

/// Returns the number of uncles in a block from a block matching the given block hash.
///
/// It will always return 0 since Tendermint doesn't have uncles.
pub async fn get_uncle_count_by_block_hash<C>(
    _data: JsonRpcData<C>,
    _params: Params<(et::H256,)>,
) -> JsonRpcResult<et::U256> {
    Ok(et::U256::zero())
}

/// Returns the number of uncles in a block from a block matching the given block number.
///
/// It will always return 0 since Tendermint doesn't have uncles.
pub async fn get_uncle_count_by_block_number<C>(
    _data: JsonRpcData<C>,
    _params: Params<(et::BlockNumber,)>,
) -> JsonRpcResult<et::U256> {
    Ok(et::U256::zero())
}

/// Returns information about a uncle of a block by hash and uncle index position.
///
/// It will always return None since Tendermint doesn't have uncles.
pub async fn get_uncle_by_block_hash_and_index<C>(
    _data: JsonRpcData<C>,
    _params: Params<(et::H256, et::U64)>,
) -> JsonRpcResult<Option<et::Block<et::H256>>> {
    Ok(None)
}

/// Returns information about a uncle of a block by number and uncle index position.
///
/// It will always return None since Tendermint doesn't have uncles.
pub async fn get_uncle_by_block_number_and_index<C>(
    _data: JsonRpcData<C>,
    _params: Params<(et::BlockNumber, et::U64)>,
) -> JsonRpcResult<Option<et::Block<et::H256>>> {
    Ok(None)
}

fn h160_to_fvm_addr(addr: et::H160) -> fvm_shared::address::Address {
    Address::from(&EthAddress(addr.0))
}
