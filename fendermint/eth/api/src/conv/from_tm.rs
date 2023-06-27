// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Helper methods to convert between Ethereum and Tendermint data formats.

use std::str::FromStr;

use anyhow::{anyhow, Context};
use cid::multihash::MultihashDigest;
use ethers_core::types::{self as et};
use fendermint_vm_actor_interface::eam::EthAddress;
use fendermint_vm_message::{chain::ChainMessage, signed::SignedMessage};
use fvm_shared::chainid::ChainID;
use fvm_shared::{bigint::BigInt, econ::TokenAmount};
use lazy_static::lazy_static;
use tendermint::abci::response::DeliverTx;
use tendermint::abci::EventAttribute;
use tendermint_rpc::endpoint;

use super::from_fvm::{to_eth_address, to_eth_signature, to_eth_tokens};

// Values taken from https://github.com/filecoin-project/lotus/blob/6e7dc9532abdb3171427347710df4c860f1957a2/chain/types/ethtypes/eth_types.go#L199

lazy_static! {
    static ref EMPTY_ETH_HASH: et::H256 = et::H256::default();

    // Keccak-256 of an RLP of an empty array
    static ref EMPTY_UNCLE_HASH: et::H256 = et::H256::from_slice(
        hex::decode("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347")
            .unwrap()
            .as_ref(),
    );

    // Keccak-256 hash of the RLP of null
    static ref EMPTY_ROOT_HASH: et::H256 = et::H256::from_slice(
        hex::decode("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")
            .unwrap()
            .as_ref(),
    );

    static ref EMPTY_ETH_BLOOM: [u8; 2048/8] = [0u8; 2048/8];
    static ref FULL_ETH_BLOOM: [u8; 2048/8] = [0xff; 2048/8];

    static ref MAX_U256: BigInt = BigInt::from_str(&et::U256::MAX.to_string()).unwrap();
}

/// Convert a Tendermint block to Ethereum with only the block hashes in the body.
pub fn to_eth_block(
    block: tendermint::Block,
    block_results: tendermint_rpc::endpoint::block_results::Response,
    base_fee: TokenAmount,
    chain_id: ChainID,
) -> anyhow::Result<et::Block<et::Transaction>> {
    // Based on https://github.com/evmos/ethermint/blob/07cf2bd2b1ce9bdb2e44ec42a39e7239292a14af/rpc/types/utils.go#L113
    //          https://github.com/evmos/ethermint/blob/07cf2bd2b1ce9bdb2e44ec42a39e7239292a14af/rpc/backend/blocks.go#L365
    //          https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L1883

    let hash = et::H256::from_slice(block.header().hash().as_ref());

    let parent_hash = block
        .header()
        .last_block_id
        .map(|id| et::H256::from_slice(id.hash.as_bytes()))
        .unwrap_or_default();

    let transactions_root = if block.data.is_empty() {
        *EMPTY_ROOT_HASH
    } else {
        block
            .header()
            .data_hash
            .map(|h| et::H256::from_slice(h.as_bytes()))
            .unwrap_or(*EMPTY_ROOT_HASH)
    };

    // Tendermint's account hash luckily has the same length as Eth.
    let author = et::H160::from_slice(block.header().proposer_address.as_bytes());

    let transaction_results = block_results.txs_results.unwrap_or_default();
    let mut transactions = Vec::new();
    let mut size = et::U256::zero();
    let mut gas_limit = et::U256::zero();
    let mut gas_used = et::U256::zero();

    // I'm just going to skip all the future message types here, which are CID based.
    // To deal with them, we'd have to send IPLD requests via ABCI to resolve them,
    // potentially through multiple hops. Let's leave that for the future and for now
    // assume that all we have is signed transactions.
    for (idx, data) in block.data().iter().enumerate() {
        size += et::U256::from(data.len());

        if let Some(result) = transaction_results.get(idx) {
            gas_used += et::U256::from(result.gas_used);
            gas_limit += et::U256::from(result.gas_wanted);
        }

        let msg = to_chain_message(data)?;

        if let ChainMessage::Signed(msg) = msg {
            let hash = message_hash(data)?;

            let mut tx = to_eth_transaction(hash, *msg, chain_id)
                .context("failed to convert to eth transaction")?;
            tx.transaction_index = Some(et::U64::from(idx));
            transactions.push(tx);
        }
    }

    let block = et::Block {
        hash: Some(hash),
        parent_hash,
        number: Some(et::U64::from(block.header().height.value())),
        timestamp: et::U256::from(block.header().time.unix_timestamp()),
        author: Some(author),
        state_root: app_hash_to_root(&block.header().app_hash)?,
        transactions_root,
        base_fee_per_gas: Some(to_eth_tokens(&base_fee)?),
        difficulty: et::U256::zero(),
        total_difficulty: None,
        nonce: None,
        mix_hash: None,
        uncles: Vec::new(),
        uncles_hash: *EMPTY_UNCLE_HASH,
        receipts_root: *EMPTY_ROOT_HASH,
        extra_data: et::Bytes::default(),
        logs_bloom: None,
        withdrawals_root: None,
        withdrawals: None,
        seal_fields: Vec::new(),
        other: Default::default(),
        transactions,
        size: Some(size),
        gas_limit,
        gas_used,
    };

    Ok(block)
}

pub fn to_eth_transaction(
    hash: tendermint::Hash,
    msg: SignedMessage,
    chain_id: ChainID,
) -> anyhow::Result<et::Transaction> {
    // Based on https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L2048
    let sig = to_eth_signature(msg.signature()).context("failed to convert to eth signature")?;
    let hash = et::H256::from_slice(hash.as_bytes());
    let msg = msg.message;

    let tx = et::Transaction {
        hash,
        nonce: et::U256::from(msg.sequence),
        block_hash: None,
        block_number: None,
        transaction_index: None,
        from: to_eth_address(&msg.from).unwrap_or_default(),
        to: to_eth_address(&msg.to),
        value: to_eth_tokens(&msg.value)?,
        gas: et::U256::from(msg.gas_limit),
        max_fee_per_gas: Some(to_eth_tokens(&msg.gas_fee_cap)?),
        max_priority_fee_per_gas: Some(to_eth_tokens(&msg.gas_premium)?),
        gas_price: None,
        input: et::Bytes::from(msg.params.bytes().to_vec()),
        chain_id: Some(et::U256::from(u64::from(chain_id))),
        v: et::U64::from(sig.v),
        r: sig.r,
        s: sig.s,
        transaction_type: None,
        access_list: None,
        other: Default::default(),
    };

    Ok(tx)
}

/// Helper function to produce cumulative gas used after the execution of each transaction in a block,
/// along with cumulative event log count.
pub fn to_cumulative(block_results: &endpoint::block_results::Response) -> Vec<(et::U256, usize)> {
    let mut records = Vec::new();
    let mut cumulative_gas_used = et::U256::zero();
    let mut cumulative_event_count = 0usize;
    if let Some(rs) = block_results.txs_results.as_ref() {
        for r in rs {
            cumulative_gas_used += et::U256::from(r.gas_used);
            cumulative_event_count += r.events.len();
            records.push((cumulative_gas_used, cumulative_event_count));
        }
    }
    records
}

// https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L2174
// https://github.com/evmos/ethermint/blob/07cf2bd2b1ce9bdb2e44ec42a39e7239292a14af/rpc/backend/tx_info.go#L147
pub fn to_eth_receipt(
    msg: &SignedMessage,
    result: &endpoint::tx::Response,
    cumulative: &[(et::U256, usize)],
    header: &tendermint::block::Header,
    base_fee: &TokenAmount,
) -> anyhow::Result<et::TransactionReceipt> {
    let block_hash = et::H256::from_slice(header.hash().as_bytes());
    let block_number = et::U64::from(result.height.value());
    let transaction_hash = et::H256::from_slice(result.hash.as_bytes());
    let transaction_index = et::U64::from(result.index);

    let msg = &msg.message;
    // Lotus effective gas price is based on total spend divided by gas used,
    // for which it recalculates the gas outputs. However, we don't have access
    // to the VM interpreter here to restore those results, and they are discarded
    // from the [`ApplyRet`] during the conversion to [`DeliverTx`].
    // We could put it into the [`DeliverTx::info`] field, or we can calculate
    // something based on the gas fields of the transaction, like Ethermint.
    let effective_gas_price =
        crate::gas::effective_gas_price(msg, base_fee, result.tx_result.gas_used);

    // Sum up gas up to this transaction.
    let (cumulative_gas_used, cumulative_event_count) = cumulative
        .get(result.index as usize)
        .cloned()
        .unwrap_or_default();

    let log_index_start = cumulative_event_count.saturating_sub(result.tx_result.events.len());

    let mut logs = Vec::new();
    for (idx, event) in result.tx_result.events.iter().enumerate() {
        // TODO: Lotus looks up an Ethereum address based on the actor ID:
        // https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L1987
        let address = event
            .attributes
            .iter()
            .find(|a| a.key == "emitter")
            .and_then(|a| a.value.parse::<u64>().ok())
            .map(EthAddress::from_id)
            .map(|a| et::H160::from_slice(&a.0))
            .unwrap_or_default();

        // https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#LL2240C9-L2240C15
        let (topics, data) = to_topics_and_data(&event.attributes)?;

        let log = et::Log {
            address,
            topics,
            data,
            block_hash: Some(block_hash),
            block_number: Some(block_number),
            transaction_hash: Some(transaction_hash),
            transaction_index: Some(transaction_index),
            log_index: Some(et::U256::from(idx + log_index_start)),
            transaction_log_index: Some(et::U256::from(idx)),
            log_type: Some(event.kind.clone()),
            removed: Some(false),
        };

        logs.push(log);
    }

    // See if the return value is an Ethereum contract creation.
    // https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#LL2240C9-L2240C15
    let contract_address = if result.tx_result.code.is_err() {
        None
    } else {
        maybe_contract_address(&result.tx_result).map(|ca| et::H160::from_slice(&ca.0))
    };

    let receipt = et::TransactionReceipt {
        transaction_hash,
        transaction_index,
        block_hash: Some(block_hash),
        block_number: Some(block_number),
        from: to_eth_address(&msg.from).unwrap_or_default(),
        to: to_eth_address(&msg.to),
        cumulative_gas_used,
        gas_used: Some(et::U256::from(result.tx_result.gas_used)),
        contract_address,
        logs,
        status: Some(et::U64::from(if result.tx_result.code.is_ok() {
            1
        } else {
            0
        })),
        root: Some(app_hash_to_root(&header.app_hash)?),
        logs_bloom: et::Bloom::from_slice(&*EMPTY_ETH_BLOOM),
        transaction_type: Some(et::U64::from(2)), // Value used by Lotus.
        effective_gas_price: Some(to_eth_tokens(&effective_gas_price)?),
        other: Default::default(),
    };
    Ok(receipt)
}

/// Change the type of transactions in a block by mapping a function over them.
pub fn map_rpc_block_txs<F, A, B, E>(block: et::Block<A>, f: F) -> Result<et::Block<B>, E>
where
    F: Fn(A) -> Result<B, E>,
{
    let et::Block {
        hash,
        parent_hash,
        uncles_hash,
        author,
        state_root,
        transactions_root,
        receipts_root,
        number,
        gas_used,
        gas_limit,
        extra_data,
        logs_bloom,
        timestamp,
        difficulty,
        total_difficulty,
        seal_fields,
        uncles,
        transactions,
        size,
        mix_hash,
        nonce,
        base_fee_per_gas,
        withdrawals_root,
        withdrawals,
        other,
    } = block;

    let transactions: Result<Vec<B>, E> = transactions.into_iter().map(f).collect();
    let transactions = transactions?;

    let block = et::Block {
        hash,
        parent_hash,
        uncles_hash,
        author,
        state_root,
        transactions_root,
        receipts_root,
        number,
        gas_used,
        gas_limit,
        extra_data,
        logs_bloom,
        timestamp,
        difficulty,
        total_difficulty,
        seal_fields,
        uncles,
        size,
        mix_hash,
        nonce,
        base_fee_per_gas,
        withdrawals_root,
        withdrawals,
        transactions,
        other,
    };

    Ok(block)
}

fn app_hash_to_root(app_hash: &tendermint::AppHash) -> anyhow::Result<et::H256> {
    // Out app hash is a CID. We only need the hash part.
    let state_root = cid::Cid::try_from(app_hash.as_bytes())?;
    let state_root = et::H256::from_slice(state_root.hash().digest());
    Ok(state_root)
}

fn maybe_contract_address(deliver_tx: &DeliverTx) -> Option<EthAddress> {
    fendermint_rpc::response::decode_fevm_create(deliver_tx)
        .ok()
        .map(|cr| cr.eth_address)
}

// Find the Ethereum topics (up to 4) and the data in the event attributes.
fn to_topics_and_data(attrs: &Vec<EventAttribute>) -> anyhow::Result<(Vec<et::H256>, et::Bytes)> {
    // Based on https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L1534
    let mut topics = Vec::new();
    let mut data = None;
    for attr in attrs {
        let bz = hex::decode(&attr.value)?;
        match attr.key.as_str() {
            "t1" | "t2" | "t3" | "t4" => {
                if bz.len() != 32 {
                    return Err(anyhow!("unexpected topic value: {attr:?}"));
                }
                let h = et::H256::from_slice(&bz);
                let i = attr.key[1..].parse::<usize>().unwrap().saturating_sub(1);
                while topics.len() <= i {
                    topics.push(et::H256::default())
                }
                topics[i] = h;
            }
            "d" => {
                data = Some(et::Bytes::from_iter(bz.iter()));
            }
            _ => {}
        }
    }
    Ok((topics, data.unwrap_or_default()))
}

/// Decode the transaction payload as a [ChainMessage].
pub fn to_chain_message(tx: &[u8]) -> anyhow::Result<ChainMessage> {
    fvm_ipld_encoding::from_slice::<ChainMessage>(tx).context("failed to decode tx as ChainMessage")
}

/// Hash the transaction payload for Tendermint.
pub fn message_hash(tx: &[u8]) -> anyhow::Result<tendermint::Hash> {
    let hash = cid::multihash::Code::Sha3_256.digest(tx);
    tendermint::Hash::from_bytes(tendermint::hash::Algorithm::Sha256, hash.digest())
        .context("failed to convert to tendermint hash")
}
