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
use tendermint::block::Height;
use tendermint_rpc::{
    endpoint::{block, commit, header, header_by_hash},
    Client,
};

use crate::{JsonRpcData, JsonRpcResult};

/// Get the Tendermint block at a specific height.
async fn tm_block_by_height<C>(
    client: &C,
    block_number: ethtypes::BlockNumber,
) -> JsonRpcResult<tendermint::Block>
where
    C: Client + Sync,
{
    let block = match block_number {
        ethtypes::BlockNumber::Number(height) => {
            let height = Height::try_from(height.as_u64())?;
            let res: block::Response = client.block(height).await?;
            res.block
        }
        ethtypes::BlockNumber::Finalized
        | ethtypes::BlockNumber::Latest
        | ethtypes::BlockNumber::Safe
        | ethtypes::BlockNumber::Pending => {
            let res: block::Response = client.latest_block().await?;
            res.block
        }
        ethtypes::BlockNumber::Earliest => {
            let res: block::Response = client.block(Height::from(1u32)).await?;
            res.block
        }
    };
    Ok(block)
}

/// Get the Tendermint header at a specific height.
async fn tm_header_by_height<C>(
    client: &C,
    block_number: ethtypes::BlockNumber,
) -> JsonRpcResult<tendermint::block::Header>
where
    C: Client + Sync,
{
    let header = match block_number {
        ethtypes::BlockNumber::Number(height) => {
            let height = Height::try_from(height.as_u64())?;
            let res: header::Response = client.header(height).await?;
            res.header
        }
        ethtypes::BlockNumber::Finalized
        | ethtypes::BlockNumber::Latest
        | ethtypes::BlockNumber::Safe
        | ethtypes::BlockNumber::Pending => {
            let res: commit::Response = client.latest_commit().await?;
            res.signed_header.header
        }
        ethtypes::BlockNumber::Earliest => {
            let res: header::Response = client.header(Height::from(1u32)).await?;
            res.header
        }
    };
    Ok(header)
}

/// Get the Tendermint header by hash
async fn tm_header_by_hash<C>(
    client: &C,
    block_hash: ethtypes::H256,
) -> JsonRpcResult<tendermint::block::Header>
where
    C: Client + Sync,
{
    let hash = tendermint::Hash::Sha256(*block_hash.as_fixed_bytes());
    let res: header_by_hash::Response = client.header_by_hash(hash).await?;
    match res.header {
        Some(header) => Ok(header),
        None => Err(jsonrpc_v2::Error::Full {
            code: ExitCode::USR_NOT_FOUND.code(),
            message: format!("block {block_hash} not found"),
            data: None,
        }),
    }
}

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
        BlockId::Number(n) => tm_header_by_height(data.client.underlying(), n).await?,
        BlockId::Hash(h) => tm_header_by_hash(data.client.underlying(), h).await?,
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
    Params(block_number): Params<ethtypes::BlockNumber>,
) -> JsonRpcResult<ethtypes::U64>
where
    C: Client + Sync,
{
    let block = tm_block_by_height(data.client.underlying(), block_number).await?;

    Ok(ethtypes::U64::from(block.data.len()))
}
