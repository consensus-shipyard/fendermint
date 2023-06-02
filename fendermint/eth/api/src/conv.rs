// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Helper methods to convert between Ethereum and Tendermint data formats.

use std::str::FromStr;

use anyhow::anyhow;
use ethers_core::types::{self as et};
use fendermint_vm_actor_interface::eam::EthAddress;
use fendermint_vm_actor_interface::eam::EAM_ACTOR_ID;
use fendermint_vm_message::{chain::ChainMessage, signed::SignedMessage};
use fvm_shared::chainid::ChainID;
use fvm_shared::crypto::signature::SignatureType;
use fvm_shared::crypto::signature::SECP_SIG_LEN;
use fvm_shared::{address::Payload, bigint::BigInt, econ::TokenAmount};
use lazy_static::lazy_static;
use libsecp256k1::RecoveryId;

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

    // Out app hash is a CID. We only need the hash part.
    let state_root = cid::Cid::try_from(block.header().app_hash.as_bytes())?;
    let state_root = et::H256::from_slice(state_root.hash().digest());

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
        let msg = fvm_ipld_encoding::from_slice::<ChainMessage>(data)?;
        if let ChainMessage::Signed(msg) = msg {
            let hash = tendermint::Hash::from_bytes(tendermint::hash::Algorithm::Sha256, data)?;
            let mut tx = to_rpc_transaction(hash, *msg, chain_id)?;
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
        state_root,
        transactions_root,
        base_fee_per_gas: Some(tokens_to_u256(&base_fee)?),
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

pub fn to_rpc_transaction(
    hash: tendermint::Hash,
    msg: SignedMessage,
    chain_id: ChainID,
) -> anyhow::Result<et::Transaction> {
    // Based on https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L2048
    let sig = msg.signature;
    let (v, sig) = match sig.sig_type {
        SignatureType::Secp256k1 => parse_secp256k1(&sig.bytes)?,
        other => return Err(anyhow!("unexpected signature type: {other:?}")),
    };

    let msg = msg.message;

    // The following hash is what we use during signing, however, it would be useless
    // when trying to look up the transaction in the Tendermint API.
    // Judging by the parameters of the `tendermint_rpc::Client::tx` method Tendermint
    // probably uses a SHA256 hash of the data to index transactions.
    // let cid = SignedMessage::cid(&msg)?;
    // let hash = et::H256::from_slice(cid.hash().digest());
    let hash = et::H256::from_slice(hash.as_bytes());

    let from = match msg.from.payload() {
        Payload::Secp256k1(h) => et::H160::from_slice(h),
        Payload::Delegated(d) if d.namespace() == EAM_ACTOR_ID && d.subaddress().len() == 20 => {
            et::H160::from_slice(d.subaddress())
        }
        other => return Err(anyhow!("unexpected `from` address payload: {other:?}")),
    };

    let to = match msg.to.payload() {
        Payload::Secp256k1(h) => Some(et::H160::from_slice(h)),
        Payload::Delegated(d) if d.namespace() == EAM_ACTOR_ID && d.subaddress().len() == 20 => {
            Some(et::H160::from_slice(d.subaddress()))
        }
        Payload::Actor(h) => Some(et::H160::from_slice(h)),
        Payload::ID(id) => Some(et::H160::from_slice(&EthAddress::from_id(*id).0)),
        _ => None, // BLS or an invalid delegated address. Just move on.
    };

    let tx = et::Transaction {
        hash,
        nonce: et::U256::from(msg.sequence),
        block_hash: None,
        block_number: None,
        transaction_index: None,
        from,
        to,
        value: tokens_to_u256(&msg.value)?,
        gas: et::U256::from(msg.gas_limit),
        max_fee_per_gas: Some(tokens_to_u256(&msg.gas_fee_cap)?),
        max_priority_fee_per_gas: Some(tokens_to_u256(&msg.gas_premium)?),
        gas_price: None,
        input: et::Bytes::from(msg.params.bytes().to_vec()),
        chain_id: Some(et::U256::from(u64::from(chain_id))),
        v: et::U64::from(v.serialize()),
        r: et::U256::from_big_endian(sig.r.b32().as_ref()),
        s: et::U256::from_big_endian(sig.s.b32().as_ref()),
        transaction_type: None,
        access_list: None,
        other: Default::default(),
    };

    Ok(tx)
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

pub fn tokens_to_u256(amount: &TokenAmount) -> anyhow::Result<et::U256> {
    if amount.atto() > &MAX_U256 {
        Err(anyhow!("TokenAmount > U256.MAX"))
    } else {
        let (_sign, bz) = amount.atto().to_bytes_be();
        Ok(et::U256::from_big_endian(&bz))
    }
}

fn parse_secp256k1(
    sig: &[u8],
) -> anyhow::Result<(libsecp256k1::RecoveryId, libsecp256k1::Signature)> {
    if sig.len() != SECP_SIG_LEN {
        return Err(anyhow!("unexpected Secp256k1 length: {}", sig.len()));
    }

    // generate types to recover key from
    let rec_id = RecoveryId::parse(sig[64])?;

    // Signature value without recovery byte
    let mut s = [0u8; 64];
    s.clone_from_slice(&sig[..64]);

    // generate Signature
    let sig = libsecp256k1::Signature::parse_standard(&s)?;

    Ok((rec_id, sig))
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use fendermint_testing::arb::ArbTokenAmount;
    use fvm_shared::{bigint::BigInt, econ::TokenAmount};
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

    #[test]
    fn test_token_amount_to_u256() {
        let atto = BigInt::from_str(
            "99191064924191451313862974502415542781658129482631472725645205117646186753315",
        )
        .unwrap();

        let tokens = TokenAmount::from_atto(atto);

        tokens_to_u256(&tokens).unwrap();
    }
}
