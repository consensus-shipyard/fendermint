// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Helper methods to convert between Ethereum and Tendermint data formats.

use ethers_core::types::{self as et};

/// Convert a Tendermint block to Ethereum with only the block hashes in the body.
pub fn to_rpc_block(block: tendermint::Block) -> et::Block<et::H256> {
    // Based on https://github.com/evmos/ethermint/blob/07cf2bd2b1ce9bdb2e44ec42a39e7239292a14af/rpc/types/utils.go#L113
    //          https://github.com/evmos/ethermint/blob/07cf2bd2b1ce9bdb2e44ec42a39e7239292a14af/rpc/backend/blocks.go#L365
    //          https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L1883

    let hash = et::H256::from_slice(block.header().hash().as_ref());

    let parent_hash = block
        .header()
        .last_block_id
        .map(|id| et::H256::from_slice(id.hash.as_bytes()))
        .unwrap_or_default();

    et::Block {
        hash: Some(hash),
        parent_hash,
        uncles_hash: todo!(),
        author: todo!(),
        state_root: todo!(),
        transactions_root: todo!(),
        receipts_root: todo!(),
        number: todo!(),
        gas_used: todo!(),
        gas_limit: todo!(),
        extra_data: todo!(),
        logs_bloom: todo!(),
        timestamp: todo!(),
        difficulty: todo!(),
        total_difficulty: todo!(),
        seal_fields: todo!(),
        uncles: todo!(),
        size: todo!(),
        mix_hash: todo!(),
        nonce: todo!(),
        base_fee_per_gas: todo!(),
        withdrawals_root: todo!(),
        withdrawals: todo!(),
        transactions: todo!(),
        other: todo!(),
    }
}

/// Change the type of transactions in a block by mapping a function over them.
pub fn map_rpc_block_txs<F, A, B>(block: et::Block<A>, f: F) -> anyhow::Result<et::Block<B>>
where
    F: Fn(A) -> anyhow::Result<B>,
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

    let transactions: anyhow::Result<Vec<B>> = transactions.into_iter().map(f).collect();
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
