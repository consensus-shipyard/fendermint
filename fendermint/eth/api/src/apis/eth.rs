// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

// See the following for inspiration:
// * https://github.com/evmos/ethermint/blob/ebbe0ffd0d474abd745254dc01e60273ea758dae/rpc/namespaces/ethereum/eth/api.go#L44
// * https://github.com/filecoin-project/lotus/blob/v1.23.1-rc2/api/api_full.go#L783

use jsonrpc_v2::{Data, Error as JsonRpcError, Params};

use crate::JsonRpcState;

/// Returns a list of addresses owned by client.
///
/// It will always return [] since we don't expect Fendermint to manage private keys.
pub async fn accounts(data: Data<JsonRpcState>) -> Result<Vec<String>, JsonRpcError> {
    todo!()
}

/// Returns the number of most recent block.
pub async fn block_number(data: Data<JsonRpcState>) -> Result<u64, JsonRpcError> {
    todo!()
}

/// Returns the number of transactions in a block matching the given block number.
///
/// QUANTITY|TAG - integer of a block number, or the string "earliest", "latest" or "pending", as in the default block parameter.
pub async fn get_block_transaction_count_by_number(
    data: Data<JsonRpcState>,
    Params(params): Params<u64>,
) -> Result<u64, JsonRpcError> {
    todo!()
}
