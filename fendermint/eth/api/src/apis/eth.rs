// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

// See the following for inspiration:
// * https://github.com/evmos/ethermint/blob/ebbe0ffd0d474abd745254dc01e60273ea758dae/rpc/namespaces/ethereum/eth/api.go#L44
// * https://github.com/filecoin-project/lotus/blob/v1.23.1-rc2/api/api_full.go#L783
// * https://github.com/filecoin-project/lotus/blob/v1.23.1-rc2/node/impl/full/eth.go

use std::collections::HashSet;

use anyhow::Context;
use ethers_core::types as et;
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::utils::rlp;
use fendermint_rpc::message::MessageFactory;
use fendermint_rpc::query::QueryClient;
use fendermint_rpc::response::decode_fevm_invoke;
use fendermint_vm_actor_interface::eam::EthAddress;
use fendermint_vm_actor_interface::evm;
use fendermint_vm_message::chain::ChainMessage;
use fendermint_vm_message::signed::SignedMessage;
use fvm_ipld_encoding::RawBytes;
use fvm_shared::address::Address;
use fvm_shared::crypto::signature::Signature;
use fvm_shared::{chainid::ChainID, error::ExitCode};
use jsonrpc_v2::Params;
use serde::{Deserialize, Serialize};
use tendermint_rpc::endpoint::{self, status};
use tendermint_rpc::SubscriptionClient;
use tendermint_rpc::{
    endpoint::{block, block_results, broadcast::tx_sync, consensus_params, header},
    Client,
};

use crate::conv::from_eth::{to_fvm_message, to_tm_hash};
use crate::conv::from_tm::{self, message_hash, to_chain_message, to_cumulative};
use crate::filters::{matches_topics, FilterId, FilterKind, FilterRecords};
use crate::{
    conv::{
        from_eth::to_fvm_address,
        from_fvm::to_eth_tokens,
        from_tm::{to_eth_receipt, to_eth_transaction},
    },
    error, JsonRpcData, JsonRpcResult,
};

const MAX_FEE_HIST_SIZE: usize = 1024;

/// Returns a list of addresses owned by client.
///
/// It will always return [] since we don't expect Fendermint to manage private keys.
pub async fn accounts<C>(_data: JsonRpcData<C>) -> JsonRpcResult<Vec<et::Address>> {
    Ok(vec![])
}

/// Returns the number of most recent block.
pub async fn block_number<C>(data: JsonRpcData<C>) -> JsonRpcResult<et::U64>
where
    C: Client + Sync + Send,
{
    let res: block::Response = data.tm().latest_block().await?;
    let height = res.block.header.height;
    Ok(et::U64::from(height.value()))
}

/// Returns the chain ID used for signing replay-protected transactions.
pub async fn chain_id<C>(data: JsonRpcData<C>) -> JsonRpcResult<et::U64>
where
    C: Client + Sync + Send,
{
    let res = data.client.state_params(None).await?;
    Ok(et::U64::from(res.value.chain_id))
}

/// The current FVM network version.
pub async fn protocol_version<C>(data: JsonRpcData<C>) -> JsonRpcResult<String>
where
    C: Client + Sync + Send,
{
    let res = data.client.state_params(None).await?;
    let version: u32 = res.value.network_version.into();
    Ok(version.to_string())
}

/// Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.
pub async fn fee_history<C>(
    data: JsonRpcData<C>,
    Params((block_count, last_block, reward_percentiles)): Params<(
        et::U256,
        et::BlockNumber,
        Vec<f64>,
    )>,
) -> JsonRpcResult<et::FeeHistory>
where
    C: Client + Sync + Send,
{
    if block_count > et::U256::from(MAX_FEE_HIST_SIZE) {
        return error(
            ExitCode::USR_ILLEGAL_ARGUMENT,
            "block_count must be <= 1024",
        );
    }

    let mut hist = et::FeeHistory {
        base_fee_per_gas: Vec::new(),
        gas_used_ratio: Vec::new(),
        oldest_block: et::U256::default(),
        reward: Vec::new(),
    };
    let mut block_number = last_block;
    let mut block_count = block_count.as_usize();

    while block_count > 0 {
        let block = data
            .block_by_height(block_number)
            .await
            .context("failed to get block")?;

        let height = block.header().height;

        // Genesis has height 1, but no relevant fees.
        if height.value() <= 1 {
            break;
        }
        let state_params = data.client.state_params(Some(height)).await?;
        let base_fee = &state_params.value.base_fee;

        let consensus_params: consensus_params::Response = data
            .tm()
            .consensus_params(height)
            .await
            .context("failed to get consensus params")?;

        let mut block_gas_limit = consensus_params.consensus_params.block.max_gas;
        if block_gas_limit <= 0 {
            block_gas_limit =
                i64::try_from(fvm_shared::BLOCK_GAS_LIMIT).expect("FVM block gas limit not i64")
        };

        // The latest block might not have results yet.
        if let Ok(block_results) = data.tm().block_results(height).await {
            let txs_results = block_results.txs_results.unwrap_or_default();
            let total_gas_used: i64 = txs_results.iter().map(|r| r.gas_used).sum();

            let mut premiums = Vec::new();
            for (tx, txres) in block.data().iter().zip(txs_results) {
                let msg = fvm_ipld_encoding::from_slice::<ChainMessage>(tx)
                    .context("failed to decode tx as ChainMessage")?;

                if let ChainMessage::Signed(msg) = msg {
                    let premium = crate::gas::effective_gas_premium(&msg.message, base_fee);
                    premiums.push((premium, txres.gas_used));
                }
            }
            premiums.sort();

            let premium_gas_used: i64 = premiums.iter().map(|(_, gas)| *gas).sum();

            let rewards: Result<Vec<et::U256>, _> = reward_percentiles
                .iter()
                .map(|p| {
                    if premiums.is_empty() {
                        Ok(et::U256::zero())
                    } else {
                        let threshold_gas_used = (premium_gas_used as f64 * p / 100f64) as i64;
                        let mut sum_gas_used = 0;
                        let mut idx = 0;
                        while sum_gas_used < threshold_gas_used && idx < premiums.len() - 1 {
                            sum_gas_used += premiums[idx].1;
                            idx += 1;
                        }
                        to_eth_tokens(&premiums[idx].0)
                    }
                })
                .collect();

            hist.oldest_block = et::U256::from(height.value());
            hist.base_fee_per_gas.push(to_eth_tokens(base_fee)?);
            hist.gas_used_ratio
                .push(total_gas_used as f64 / block_gas_limit as f64);
            hist.reward.push(rewards?);
        }

        block_count -= 1;
        block_number = et::BlockNumber::Number(et::U64::from(height.value() - 1));
    }

    // Reverse data to be oldest-to-newest.
    hist.base_fee_per_gas.reverse();
    hist.gas_used_ratio.reverse();
    hist.reward.reverse();

    Ok(hist)
}

/// Returns the current price per gas in wei.
pub async fn gas_price<C>(data: JsonRpcData<C>) -> JsonRpcResult<et::U256>
where
    C: Client + Sync + Send,
{
    let res = data.client.state_params(None).await?;
    let price = to_eth_tokens(&res.value.base_fee)?;
    Ok(price)
}

/// Returns the balance of the account of given address.
pub async fn get_balance<C>(
    data: JsonRpcData<C>,
    Params((addr, block_id)): Params<(et::Address, et::BlockId)>,
) -> JsonRpcResult<et::U256>
where
    C: Client + Sync + Send,
{
    let header = data.header_by_id(block_id).await?;
    let height = header.height;
    let addr = to_fvm_address(addr);
    let res = data.client.actor_state(&addr, Some(height)).await?;

    match res.value {
        Some((_, state)) => Ok(to_eth_tokens(&state.balance)?),
        None => error(ExitCode::USR_NOT_FOUND, format!("actor {addr} not found")),
    }
}

/// Returns information about a block by hash.
pub async fn get_block_by_hash<C>(
    data: JsonRpcData<C>,
    Params((block_hash, full_tx)): Params<(et::H256, bool)>,
) -> JsonRpcResult<Option<et::Block<serde_json::Value>>>
where
    C: Client + Sync + Send,
{
    match data.block_by_hash_opt(block_hash).await? {
        Some(block) => data.enrich_block(block, full_tx).await.map(Some),
        None => Ok(None),
    }
}

/// Returns information about a block by block number.
pub async fn get_block_by_number<C>(
    data: JsonRpcData<C>,
    Params((block_number, full_tx)): Params<(et::BlockNumber, bool)>,
) -> JsonRpcResult<Option<et::Block<serde_json::Value>>>
where
    C: Client + Sync + Send,
{
    match data.block_by_height(block_number).await? {
        block if block.header().height.value() > 0 => {
            data.enrich_block(block, full_tx).await.map(Some)
        }
        _ => Ok(None),
    }
}

/// Returns the number of transactions in a block matching the given block number.
pub async fn get_block_transaction_count_by_number<C>(
    data: JsonRpcData<C>,
    Params((block_number,)): Params<(et::BlockNumber,)>,
) -> JsonRpcResult<et::U64>
where
    C: Client + Sync + Send,
{
    let block = data.block_by_height(block_number).await?;

    Ok(et::U64::from(block.data.len()))
}

/// Returns the number of transactions in a block from a block matching the given block hash.
pub async fn get_block_transaction_count_by_hash<C>(
    data: JsonRpcData<C>,
    Params((block_hash,)): Params<(et::H256,)>,
) -> JsonRpcResult<et::U64>
where
    C: Client + Sync + Send,
{
    let block = data.block_by_hash_opt(block_hash).await?;
    let count = block
        .map(|b| et::U64::from(b.data.len()))
        .unwrap_or_default();
    Ok(count)
}

/// Returns the information about a transaction requested by transaction hash.
pub async fn get_transaction_by_block_hash_and_index<C>(
    data: JsonRpcData<C>,
    Params((block_hash, index)): Params<(et::H256, et::U64)>,
) -> JsonRpcResult<Option<et::Transaction>>
where
    C: Client + Sync + Send,
{
    if let Some(block) = data.block_by_hash_opt(block_hash).await? {
        data.transaction_by_index(block, index).await
    } else {
        Ok(None)
    }
}

/// Returns the information about a transaction requested by transaction hash.
pub async fn get_transaction_by_block_number_and_index<C>(
    data: JsonRpcData<C>,
    Params((block_number, index)): Params<(et::BlockNumber, et::U64)>,
) -> JsonRpcResult<Option<et::Transaction>>
where
    C: Client + Sync + Send,
{
    let block = data.block_by_height(block_number).await?;
    data.transaction_by_index(block, index).await
}

/// Returns the information about a transaction requested by transaction hash.
pub async fn get_transaction_by_hash<C>(
    data: JsonRpcData<C>,
    Params((tx_hash,)): Params<(et::H256,)>,
) -> JsonRpcResult<Option<et::Transaction>>
where
    C: Client + Sync + Send,
{
    let hash = to_tm_hash(&tx_hash)?;

    match data.tm().tx(hash, false).await {
        Ok(res) => {
            let msg = to_chain_message(&res.tx)?;

            if let ChainMessage::Signed(msg) = msg {
                let header: header::Response = data.tm().header(res.height).await?;
                let sp = data.client.state_params(Some(res.height)).await?;
                let chain_id = ChainID::from(sp.value.chain_id);
                let mut tx = to_eth_transaction(hash, *msg, chain_id)?;
                tx.transaction_index = Some(et::U64::from(res.index));
                tx.block_hash = Some(et::H256::from_slice(header.header.hash().as_bytes()));
                tx.block_number = Some(et::U64::from(res.height.value()));
                Ok(Some(tx))
            } else {
                error(ExitCode::USR_ILLEGAL_ARGUMENT, "incompatible transaction")
            }
        }
        Err(e) if e.to_string().contains("not found") => Ok(None),
        Err(e) => error(ExitCode::USR_UNSPECIFIED, e),
    }
}

/// Returns the number of transactions sent from an address, up to a specific block.
///
/// This is done by looking up the nonce of the account.
pub async fn get_transaction_count<C>(
    data: JsonRpcData<C>,
    Params((addr, block_id)): Params<(et::Address, et::BlockId)>,
) -> JsonRpcResult<et::U64>
where
    C: Client + Sync + Send,
{
    let header = data.header_by_id(block_id).await?;
    let height = header.height;
    let addr = to_fvm_address(addr);
    let res = data.client.actor_state(&addr, Some(height)).await?;

    match res.value {
        Some((_, state)) => {
            let nonce = state.sequence;
            Ok(et::U64::from(nonce))
        }
        None => error(ExitCode::USR_NOT_FOUND, format!("actor {addr} not found")),
    }
}

/// Returns the receipt of a transaction by transaction hash.
pub async fn get_transaction_receipt<C>(
    data: JsonRpcData<C>,
    Params((tx_hash,)): Params<(et::H256,)>,
) -> JsonRpcResult<Option<et::TransactionReceipt>>
where
    C: Client + Sync + Send,
{
    let hash = to_tm_hash(&tx_hash)?;

    match data.tm().tx(hash, false).await {
        Ok(res) => {
            let header: header::Response = data.tm().header(res.height).await?;
            let block_results: block_results::Response =
                data.tm().block_results(res.height).await?;
            let cumulative = to_cumulative(&block_results);
            let state_params = data.client.state_params(Some(res.height)).await?;
            let msg = to_chain_message(&res.tx)?;
            if let ChainMessage::Signed(msg) = msg {
                let receipt = to_eth_receipt(
                    &msg,
                    &res,
                    &cumulative,
                    &header.header,
                    &state_params.value.base_fee,
                )
                .context("failed to convert to receipt")?;

                Ok(Some(receipt))
            } else {
                error(ExitCode::USR_ILLEGAL_ARGUMENT, "incompatible transaction")
            }
        }
        Err(e) if e.to_string().contains("not found") => Ok(None),
        Err(e) => error(ExitCode::USR_UNSPECIFIED, e),
    }
}

/// Returns receipts for all the transactions in a block.
pub async fn get_block_receipts<C>(
    data: JsonRpcData<C>,
    Params((block_number,)): Params<(et::BlockNumber,)>,
) -> JsonRpcResult<Vec<et::TransactionReceipt>>
where
    C: Client + Sync + Send,
{
    let block = data.block_by_height(block_number).await?;
    let height = block.header.height;
    let state_params = data.client.state_params(Some(height)).await?;
    let block_results: block_results::Response = data.tm().block_results(height).await?;
    let cumulative = to_cumulative(&block_results);
    let mut receipts = Vec::new();

    for (index, (tx, tx_result)) in block
        .data
        .into_iter()
        .zip(block_results.txs_results.unwrap_or_default().into_iter())
        .enumerate()
    {
        let msg = to_chain_message(&tx)?;
        if let ChainMessage::Signed(msg) = msg {
            let hash = message_hash(&tx)?;

            let result = endpoint::tx::Response {
                hash,
                height,
                index: index as u32,
                tx_result,
                tx,
                proof: None,
            };

            let receipt = to_eth_receipt(
                &msg,
                &result,
                &cumulative,
                &block.header,
                &state_params.value.base_fee,
            )?;
            receipts.push(receipt)
        }
    }
    Ok(receipts)
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

/// Creates new message call transaction or a contract creation for signed transactions.
pub async fn send_raw_transaction<C>(
    data: JsonRpcData<C>,
    Params((tx,)): Params<(et::Bytes,)>,
) -> JsonRpcResult<et::TxHash>
where
    C: Client + Sync + Send,
{
    let rlp = rlp::Rlp::new(tx.as_ref());
    let (tx, sig) = TypedTransaction::decode_signed(&rlp)
        .context("failed to decode RLP as signed TypedTransaction")?;

    let msg = to_fvm_message(tx)?;
    let msg = SignedMessage {
        message: msg,
        signature: Signature::new_secp256k1(sig.to_vec()),
    };
    let msg = ChainMessage::Signed(Box::new(msg));
    let bz: Vec<u8> = MessageFactory::serialize(&msg)?;
    let res: tx_sync::Response = data.tm().broadcast_tx_sync(bz).await?;
    if res.code.is_ok() {
        Ok(et::TxHash::from_slice(res.hash.as_bytes()))
    } else {
        error(
            ExitCode::new(res.code.value()),
            hex::encode(res.data.as_ref()), // TODO: What is the content?
        )
    }
}

/// Executes a new message call immediately without creating a transaction on the block chain.
pub async fn call<C>(
    data: JsonRpcData<C>,
    Params((tx, block_id)): Params<(TypedTransaction, et::BlockId)>,
) -> JsonRpcResult<et::Bytes>
where
    C: Client + Sync + Send,
{
    let msg = to_fvm_message(tx)?;
    let header = data.header_by_id(block_id).await?;
    let response = data.client.call(msg, Some(header.height)).await?;
    let deliver_tx = response.value;

    // Based on Lotus, we should return the data from the receipt.
    if deliver_tx.code.is_err() {
        error(ExitCode::new(deliver_tx.code.value()), deliver_tx.info)
    } else {
        let return_data = decode_fevm_invoke(&deliver_tx)
            .context("error decoding data from deliver_tx in query")?;

        Ok(et::Bytes::from(return_data))
    }
}

/// The client either sends one or two items in the array, depending on whether a block ID is specified.
/// This is to keep it backwards compatible with nodes that do not support the block ID parameter.
/// If we were using `Option`, they would have to send `null`; this way it works with both 1 or 2 parameters.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum EstimateGasParams {
    One((TypedTransaction,)),
    Two((TypedTransaction, et::BlockId)),
}

/// Generates and returns an estimate of how much gas is necessary to allow the transaction to complete.
/// The transaction will not be added to the blockchain.
/// Note that the estimate may be significantly more than the amount of gas actually used by the transaction, f
/// or a variety of reasons including EVM mechanics and node performance.
pub async fn estimate_gas<C>(
    data: JsonRpcData<C>,
    Params(params): Params<EstimateGasParams>,
) -> JsonRpcResult<et::U256>
where
    C: Client + Sync + Send,
{
    let (tx, block_id) = match params {
        EstimateGasParams::One((tx,)) => (tx, et::BlockId::Number(et::BlockNumber::Latest)),
        EstimateGasParams::Two((tx, block_id)) => (tx, block_id),
    };
    let msg = to_fvm_message(tx).context("failed to convert to FVM message")?;

    let header = data
        .header_by_id(block_id)
        .await
        .context("failed to get header")?;

    let response = data
        .client
        .estimate_gas(msg, Some(header.height))
        .await
        .context("failed to call estimate gas query")?;

    let estimate = response.value;

    // Based on Lotus, we should return the data from the receipt.
    if !estimate.exit_code.is_success() {
        error(
            estimate.exit_code,
            format!("failed to estimate gas: {}", estimate.info),
        )
    } else {
        Ok(estimate.gas_limit.into())
    }
}

/// Returns the value from a storage position at a given address.
///
/// The return value is a hex encoded U256.
pub async fn get_storage_at<C>(
    data: JsonRpcData<C>,
    Params((address, position, block_id)): Params<(et::H160, et::U256, et::BlockId)>,
) -> JsonRpcResult<String>
where
    C: Client + Sync + Send,
{
    let params = evm::GetStorageAtParams {
        storage_key: {
            let mut bz = [0u8; 32];
            position.to_big_endian(&mut bz);
            evm::uints::U256::from_big_endian(&bz)
        },
    };
    let params = RawBytes::serialize(params).context("failed to serialize position to IPLD")?;

    let ret: evm::GetStorageAtReturn = data
        .read_evm_actor(address, evm::Method::GetStorageAt, params, block_id)
        .await?;

    // The client library expects hex encoded string.
    let mut bz = [0u8; 32];
    ret.storage.to_big_endian(&mut bz);
    Ok(hex::encode(bz))
}

/// Returns code at a given address.
pub async fn get_code<C>(
    data: JsonRpcData<C>,
    Params((address, block_id)): Params<(et::H160, et::BlockId)>,
) -> JsonRpcResult<et::Bytes>
where
    C: Client + Sync + Send,
{
    // This method has no input parameters.
    let params = RawBytes::default();

    let ret: evm::BytecodeReturn = data
        .read_evm_actor(address, evm::Method::GetBytecode, params, block_id)
        .await?;

    match ret.code {
        None => Ok(et::Bytes::default()),
        Some(cid) => {
            let code = data
                .client
                .ipld(&cid)
                .await
                .context("failed to fetch bytecode")?;

            Ok(code.map(et::Bytes::from).unwrap_or_default())
        }
    }
}

/// Returns an object with data about the sync status or false.
pub async fn syncing<C>(data: JsonRpcData<C>) -> JsonRpcResult<et::SyncingStatus>
where
    C: Client + Sync + Send,
{
    let status: status::Response = data.tm().status().await.context("failed to fetch status")?;
    let info = status.sync_info;
    let status = if !info.catching_up {
        et::SyncingStatus::IsFalse
    } else {
        let progress = et::SyncProgress {
            // This would be the block we executed.
            current_block: et::U64::from(info.latest_block_height.value()),
            // This would be the block we know about but haven't got to yet.
            highest_block: et::U64::from(info.latest_block_height.value()),
            // This would be the block we started syncing from.
            starting_block: Default::default(),
            pulled_states: None,
            known_states: None,
            healed_bytecode_bytes: None,
            healed_bytecodes: None,
            healed_trienode_bytes: None,
            healed_trienodes: None,
            healing_bytecode: None,
            healing_trienodes: None,
            synced_account_bytes: None,
            synced_accounts: None,
            synced_bytecode_bytes: None,
            synced_bytecodes: None,
            synced_storage: None,
            synced_storage_bytes: None,
        };
        et::SyncingStatus::IsSyncing(Box::new(progress))
    };

    Ok(status)
}

/// Returns an array of all logs matching a given filter object.
pub async fn get_logs<C>(
    data: JsonRpcData<C>,
    Params((filter,)): Params<(et::Filter,)>,
) -> JsonRpcResult<Vec<et::Log>>
where
    C: Client + Sync + Send,
{
    let (from_height, to_height) = match filter.block_option {
        et::FilterBlockOption::Range {
            from_block,
            to_block,
        } => {
            let from_block = from_block.unwrap_or_default();
            let to_block = to_block.unwrap_or_default();
            let to_header = data.header_by_height(to_block).await?;
            let from_header = if from_block == to_block {
                to_header.clone()
            } else {
                data.header_by_height(from_block).await?
            };
            (from_header.height, to_header.height)
        }
        et::FilterBlockOption::AtBlockHash(block_hash) => {
            let header = data.header_by_hash(block_hash).await?;
            (header.height, header.height)
        }
    };

    let addrs = match &filter.address {
        Some(et::ValueOrArray::Value(addr)) => vec![*addr],
        Some(et::ValueOrArray::Array(addrs)) => addrs.clone(),
        None => Vec::new(),
    };
    let addrs = addrs
        .into_iter()
        .map(|addr| Address::from(EthAddress(addr.0)))
        .collect::<HashSet<_>>();

    let mut height = from_height;
    let mut logs = Vec::new();

    while height <= to_height {
        if let Ok(block_results) = data.tm().block_results(height).await {
            if let Some(tx_results) = block_results.txs_results {
                let block_number = et::U64::from(height.value());

                let block = data
                    .block_by_height(et::BlockNumber::Number(block_number))
                    .await?;

                let block_hash = et::H256::from_slice(block.header().hash().as_bytes());

                let mut log_index_start = 0usize;
                for ((tx_idx, tx_result), tx) in tx_results.iter().enumerate().zip(block.data()) {
                    let tx_hash = from_tm::message_hash(tx)?;
                    let tx_hash = et::H256::from_slice(tx_hash.as_bytes());
                    let tx_idx = et::U64::from(tx_idx);

                    // Filter by address.
                    if !addrs.is_empty() {
                        if let Ok(ChainMessage::Signed(msg)) = to_chain_message(tx) {
                            if !addrs.contains(&msg.message().from) {
                                continue;
                            }
                        }
                    }

                    let mut tx_logs = from_tm::to_logs(
                        &tx_result.events,
                        block_hash,
                        block_number,
                        tx_hash,
                        tx_idx,
                        log_index_start,
                    )?;

                    // Filter by topic.
                    tx_logs.retain(|log| matches_topics(&filter, log));

                    logs.append(&mut tx_logs);

                    log_index_start += tx_result.events.len();
                }
            }
        } else {
            break;
        }
        height = height.increment()
    }

    Ok(logs)
}

/// Creates a filter object, based on filter options, to notify when the state changes (logs).
/// To check if the state has changed, call eth_getFilterChanges.
pub async fn new_filter<C>(
    data: JsonRpcData<C>,
    Params((filter,)): Params<(et::Filter,)>,
) -> JsonRpcResult<FilterId>
where
    C: SubscriptionClient + Sync + Send,
{
    let id = data
        .new_filter(FilterKind::Logs(Box::new(filter)))
        .await
        .context("failed to add log filter")?;
    Ok(id)
}

/// Creates a filter in the node, to notify when a new block arrives.
/// To check if the state has changed, call eth_getFilterChanges.
pub async fn new_block_filter<C>(data: JsonRpcData<C>) -> JsonRpcResult<FilterId>
where
    C: SubscriptionClient + Sync + Send,
{
    let id = data
        .new_filter(FilterKind::NewBlocks)
        .await
        .context("failed to add block filter")?;
    Ok(id)
}

/// Creates a filter in the node, to notify when new pending transactions arrive.
/// To check if the state has changed, call eth_getFilterChanges.
pub async fn new_pending_transaction_filter<C>(data: JsonRpcData<C>) -> JsonRpcResult<FilterId>
where
    C: SubscriptionClient + Sync + Send,
{
    let id = data
        .new_filter(FilterKind::PendingTransactions)
        .await
        .context("failed to add transaction filter")?;
    Ok(id)
}

/// Uninstalls a filter with given id. Should always be called when watch is no longer needed.
/// Additionally Filters timeout when they aren't requested with eth_getFilterChanges for a period of time
pub async fn uninstall_filter<C>(
    data: JsonRpcData<C>,
    Params((filter_id,)): Params<(FilterId,)>,
) -> JsonRpcResult<bool> {
    Ok(data.uninstall_filter(filter_id))
}

pub async fn get_filter_changes<C>(
    data: JsonRpcData<C>,
    Params((filter_id,)): Params<(FilterId,)>,
) -> JsonRpcResult<Vec<serde_json::Value>> {
    fn to_json<R: Serialize>(values: Vec<R>) -> JsonRpcResult<Vec<serde_json::Value>> {
        let values: Vec<serde_json::Value> = values
            .into_iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()
            .context("failed to convert events to JSON")?;

        Ok(values)
    }

    if let Some(accum) = data.take_filter_changes(filter_id)? {
        match accum {
            FilterRecords::Logs(logs) => to_json(logs),
            FilterRecords::NewBlocks(hashes) => to_json(hashes),
            FilterRecords::PendingTransactions(hashes) => to_json(hashes),
        }
    } else {
        error(ExitCode::USR_NOT_FOUND, "filter not found")
    }
}

pub async fn get_filter_logs<C>(
    data: JsonRpcData<C>,
    Params((filter_id,)): Params<(FilterId,)>,
) -> JsonRpcResult<Vec<et::Log>> {
    if let Some(accum) = data.take_filter_changes(filter_id)? {
        match accum {
            FilterRecords::Logs(logs) => Ok(logs),
            FilterRecords::NewBlocks(_) | FilterRecords::PendingTransactions(_) => {
                error(ExitCode::USR_ILLEGAL_STATE, "not a log filter")
            }
        }
    } else {
        error(ExitCode::USR_NOT_FOUND, "filter not found")
    }
}
