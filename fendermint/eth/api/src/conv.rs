// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Helper methods to convert between Ethereum and Tendermint data formats.

use std::str::FromStr;

use anyhow::anyhow;
use ethers_core::types::{self as et};
use fvm_shared::{bigint::BigInt, econ::TokenAmount};
use lazy_static::lazy_static;

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
pub fn to_rpc_block(
    block: tendermint::Block,
    base_fee: TokenAmount,
) -> anyhow::Result<et::Block<et::H256>> {
    // Based on https://github.com/evmos/ethermint/blob/07cf2bd2b1ce9bdb2e44ec42a39e7239292a14af/rpc/types/utils.go#L113
    //          https://github.com/evmos/ethermint/blob/07cf2bd2b1ce9bdb2e44ec42a39e7239292a14af/rpc/backend/blocks.go#L365
    //          https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L1883

    let hash = et::H256::from_slice(block.header().hash().as_ref());

    let parent_hash = block
        .header()
        .last_block_id
        .map(|id| et::H256::from_slice(id.hash.as_bytes()))
        .unwrap_or_default();

    // Out app hash is a CID, it needs to be hashed first.
    let state_root = tendermint::Hash::from_bytes(
        tendermint::hash::Algorithm::Sha256,
        block.header().app_hash.as_bytes(),
    )?;
    let state_root = et::H256::from_slice(state_root.as_bytes());

    let transactions_root = if block.data.is_empty() {
        *EMPTY_ROOT_HASH
    } else {
        block
            .header()
            .data_hash
            .map(|h| et::H256::from_slice(h.as_bytes()))
            .unwrap_or_default()
    };

    let block = et::Block {
        hash: Some(hash),
        parent_hash,
        number: Some(et::U64::from(block.header().height.value())),
        timestamp: et::U256::from(block.header().time.unix_timestamp()),
        state_root,
        transactions_root,
        base_fee_per_gas: Some(tokens_to_u256(&base_fee)?),
        difficulty: et::U256::zero(),
        total_difficulty: None,
        nonce: None,
        mix_hash: None,
        uncles: Vec::new(),
        uncles_hash: *EMPTY_UNCLE_HASH,
        extra_data: et::Bytes::default(),
        logs_bloom: None,
        withdrawals_root: None,
        withdrawals: None,
        seal_fields: Vec::new(),
        other: Default::default(),
        author: todo!(),
        transactions: todo!(),
        receipts_root: todo!(),
        gas_used: todo!(),
        gas_limit: todo!(),
        size: todo!(),
    };

    Ok(block)
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

pub fn tokens_to_u256(amount: &TokenAmount) -> anyhow::Result<et::U256> {
    if amount.atto() > &MAX_U256 {
        Err(anyhow!("TokenAmount > U256.MAX"))
    } else {
        let bz = amount.atto().to_signed_bytes_be();
        Ok(et::U256::from_big_endian(&bz))
    }
}

#[cfg(test)]
mod tests {

    use fendermint_testing::arb::ArbTokenAmount;
    use quickcheck_macros::quickcheck;

    use super::tokens_to_u256;

    #[quickcheck]
    fn prop_token_amount_to_u256(tokens: ArbTokenAmount) -> bool {
        let tokens = tokens.0;
        if let Ok(u256_from_tokens) = tokens_to_u256(&tokens) {
            let tokens_as_str = tokens.atto().to_str_radix(10);
            let u256_from_str = ethers_core::types::U256::from_dec_str(&tokens_as_str).unwrap();
            return u256_from_str == u256_from_tokens;
        }
        true
    }
}
