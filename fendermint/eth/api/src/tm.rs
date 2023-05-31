// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Tendermint RPC helper methods for the implementation of the APIs.

use ethers_core::types::{self as ethtypes};
use fvm_shared::error::ExitCode;
use jsonrpc_v2::ErrorLike;
use tendermint::block::Height;
use tendermint_rpc::{
    endpoint::{block, block_by_hash, commit, header, header_by_hash},
    Client,
};

use crate::JsonRpcResult;

/// Get the Tendermint block at a specific height.
pub async fn block_by_height<C>(
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
pub async fn header_by_height<C>(
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

/// Get a Tendermint block by hash, if it exists.
pub async fn block_by_hash_opt<C>(
    client: &C,
    block_hash: ethtypes::H256,
) -> JsonRpcResult<Option<tendermint::block::Block>>
where
    C: Client + Sync,
{
    let hash = tendermint::Hash::Sha256(*block_hash.as_fixed_bytes());
    let res: block_by_hash::Response = client.block_by_hash(hash).await?;
    Ok(res.block)
}

/// Get a Tendermint height by hash, if it exists.
pub async fn header_by_hash_opt<C>(
    client: &C,
    block_hash: ethtypes::H256,
) -> JsonRpcResult<Option<tendermint::block::Header>>
where
    C: Client + Sync,
{
    let hash = tendermint::Hash::Sha256(*block_hash.as_fixed_bytes());
    let res: header_by_hash::Response = client.header_by_hash(hash).await?;
    Ok(res.header)
}

/// Get a Tendermint header by hash.
pub async fn header_by_hash<C>(
    client: &C,
    block_hash: ethtypes::H256,
) -> JsonRpcResult<tendermint::block::Header>
where
    C: Client + Sync,
{
    match header_by_hash_opt(client, block_hash).await? {
        Some(header) => Ok(header),
        None => Err(jsonrpc_v2::Error::Full {
            code: ExitCode::USR_NOT_FOUND.code(),
            message: format!("block {block_hash} not found"),
            data: None,
        }),
    }
}
