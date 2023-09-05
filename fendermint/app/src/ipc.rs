// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! IPC related execution

use crate::app::{AppState, AppStoreKey};
use crate::BlockHeight;
use anyhow::{anyhow, Context};
use base64::Engine;
use fendermint_storage::{Codec, Encode, KVRead, KVReadable, KVStore};
use fendermint_vm_actor_interface::{ipc, system};
use fendermint_vm_interpreter::fvm::state::{FvmExecState, FvmStateParams};
use fendermint_vm_interpreter::fvm::FvmMessage;
use fendermint_vm_ipc_actors::gateway_getter_facet;
use fendermint_vm_topdown::convert::{DecodeFunctionReturn, EncodeWithSignature};
use fendermint_vm_topdown::IPCParentFinality;
use fvm::engine::MultiEngine;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::{BytesDe, RawBytes};
use fvm_shared::clock::ChainEpoch;
use fvm_shared::econ::TokenAmount;
use num_traits::Zero;
use std::sync::Arc;

/// Queries the LATEST COMMITTED parent finality from the storage
pub struct ParentFinalityQuery<DB, SS, S>
where
    SS: Blockstore + 'static,
    S: KVStore,
{
    /// Database backing all key-value operations.
    db: Arc<DB>,
    /// State store, backing all the smart contracts.
    ///
    /// Must be kept separate from storage that can be influenced by network operations such as Bitswap;
    /// nodes must be able to run transactions deterministically. By contrast the Bitswap store should
    /// be able to read its own storage area as well as state storage, to serve content from both.
    state_store: Arc<SS>,
    /// Wasm engine cache.
    multi_engine: Arc<MultiEngine>,
    /// Namespace to store app state.
    namespace: S::Namespace,
}

impl<DB, SS, S> ParentFinalityQuery<DB, SS, S>
where
    S: KVStore
        + Codec<AppState>
        + Encode<AppStoreKey>
        + Encode<BlockHeight>
        + Codec<FvmStateParams>,
    DB: KVReadable<S> + Clone + 'static,
    SS: Blockstore + Clone + 'static,
{
    pub fn new(
        db: Arc<DB>,
        state_store: Arc<SS>,
        multi_engine: Arc<MultiEngine>,
        namespace: S::Namespace,
    ) -> Self {
        Self {
            db,
            state_store,
            multi_engine,
            namespace,
        }
    }

    pub fn get_committed_finality(&self) -> anyhow::Result<IPCParentFinality> {
        let app_state = self
            .get_committed_state()?
            .ok_or_else(|| anyhow!("cannot get app state"))?;
        let block_height = app_state.block_height as ChainEpoch;
        let state_params = app_state.state_params;

        let mut exec_state = FvmExecState::new(
            self.state_store.clone(),
            self.multi_engine.as_ref(),
            block_height,
            state_params,
        )
        .context("error creating execution state")?;

        let msg = get_parent_finality_to_fvm(block_height)?;
        let (apply_ret, _) = exec_state.execute_implicit(msg)?;
        let data = apply_ret
            .msg_receipt
            .return_data
            .to_vec();
        let decoded = decode_fevm_invoke(data)?;
        DecodeFunctionReturn::<IPCParentFinality>::decode(decoded)
    }

    /// Get the last committed state, if exists.
    fn get_committed_state(&self) -> anyhow::Result<Option<AppState>> {
        let tx = self.db.read();
        tx.get(&self.namespace, &AppStoreKey::State)
            .context("get failed")
    }
}

/// Parse fvm invoke return data to the internal bytes
fn decode_fevm_invoke(bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let data = decode_data(bytes)?;

    // Some calls like transfers between Ethereum accounts don't return any data.
    if data.is_empty() {
        return Ok(data);
    }

    // This is the data return by the FEVM itself, not something wrapping another piece,
    // that is, it's as if it was returning `CreateReturn`, it's returning `RawBytes` encoded as IPLD.
    fvm_ipld_encoding::from_slice::<BytesDe>(&data)
        .map(|bz| bz.0)
        .map_err(|e| anyhow!("failed to deserialize bytes returned by FEVM: {e}"))
}

/// Decode fvm return base64 encoded string bytes
fn decode_data(data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let b64 =
        String::from_utf8(data).context("error parsing data as base64 string in ipc finality")?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .context("error parsing base64 to bytes in ipc finality")?;
    Ok(data)
}

/// Convert a get parent finality to fvm message
fn get_parent_finality_to_fvm(height: ChainEpoch) -> anyhow::Result<FvmMessage> {
    let params = RawBytes::new(EncodeWithSignature::<
        gateway_getter_facet::GetParentFinalityCall,
    >::encode(height)?);
    let msg = FvmMessage {
        version: 0,
        from: system::SYSTEM_ACTOR_ADDR,
        to: ipc::GATEWAY_ACTOR_ADDR,
        value: TokenAmount::zero(),
        method_num: ipc::gateway::METHOD_INVOKE_CONTRACT,
        params,
        // we are sending a implicit message, no need to set sequence
        sequence: 0, // read the latest

        // FIXME: what's this value?
        gas_limit: 0,
        gas_fee_cap: TokenAmount::zero(),
        gas_premium: TokenAmount::zero(),
    };

    Ok(msg)
}
