// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

// See the following for inspiration:
// * https://github.com/evmos/ethermint/blob/ebbe0ffd0d474abd745254dc01e60273ea758dae/rpc/namespaces/ethereum/eth/api.go#L44
// * https://github.com/filecoin-project/lotus/blob/v1.23.1-rc2/api/api_full.go#L783

use ethers_core::types as ethtypes;
use jsonrpc_v2::{Data, Params};
use tendermint_rpc::{endpoint, Client};

use crate::JsonRpcState;

use super::JsonRpcResult;

/// Returns a list of addresses owned by client.
///
/// It will always return [] since we don't expect Fendermint to manage private keys.
pub async fn accounts(_data: Data<JsonRpcState>) -> JsonRpcResult<Vec<ethtypes::Address>> {
    Ok(vec![])
}

/// Returns the number of most recent block.
pub async fn block_number(data: Data<JsonRpcState>) -> JsonRpcResult<ethtypes::U64> {
    let res: endpoint::block::Response = data.client.latest_block().await?;
    let height = res.block.header.height;
    let height = ethtypes::U64::from(height.value());
    Ok(height)
}

/// Returns the number of transactions in a block matching the given block number.
///
/// QUANTITY|TAG - integer of a block number, or the string "earliest", "latest" or "pending", as in the default block parameter.
pub async fn get_block_transaction_count_by_number(
    _data: Data<JsonRpcState>,
    Params(_params): Params<ethtypes::BlockNumber>,
) -> JsonRpcResult<ethtypes::U64> {
    todo!()
}
