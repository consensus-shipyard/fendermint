// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! IPC related execution

use crate::app::{AppState, AppStoreKey};
use crate::{App, BlockHeight};
use anyhow::anyhow;
use fendermint_storage::{Codec, Encode, KVReadable, KVStore, KVWritable};
use fendermint_vm_actor_interface::{ipc, system};
use fendermint_vm_interpreter::fvm::state::FvmStateParams;
use fendermint_vm_interpreter::fvm::FvmMessage;
use fendermint_vm_topdown::convert::{
    decode_parent_finality_return, encode_get_latest_parent_finality,
};
use fendermint_vm_topdown::sync::ParentFinalityStateQuery;
use fendermint_vm_topdown::IPCParentFinality;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::{BytesDe, BytesSer, RawBytes};
use fvm_shared::econ::TokenAmount;
use num_traits::Zero;

/// Queries the LATEST COMMITTED parent finality from the storage
pub struct AppParentFinalityQuery<DB, SS, S, I>
where
    SS: Blockstore + 'static,
    S: KVStore,
{
    /// The app to get state
    app: App<DB, SS, S, I>,
}

impl<DB, SS, S, I> AppParentFinalityQuery<DB, SS, S, I>
where
    S: KVStore
        + Codec<AppState>
        + Encode<AppStoreKey>
        + Encode<BlockHeight>
        + Codec<FvmStateParams>,
    DB: KVWritable<S> + KVReadable<S> + 'static + Clone,
    SS: Blockstore + 'static + Clone,
{
    pub fn new(app: App<DB, SS, S, I>) -> Self {
        Self { app }
    }
}

impl<DB, SS, S, I> ParentFinalityStateQuery for AppParentFinalityQuery<DB, SS, S, I>
where
    S: KVStore
        + Codec<AppState>
        + Encode<AppStoreKey>
        + Encode<BlockHeight>
        + Codec<FvmStateParams>,
    DB: KVWritable<S> + KVReadable<S> + 'static + Clone,
    SS: Blockstore + 'static + Clone,
{
    fn get_latest_committed_finality(&self) -> anyhow::Result<Option<IPCParentFinality>> {
        let maybe_exec_state = self.app.new_read_only_exec_state()?;

        let finality = if let Some(mut exec_state) = maybe_exec_state {
            let evm_params = encode_get_latest_parent_finality()?;
            tracing::debug!("raw evm param bytes: {}", hex::encode(&evm_params));

            let msg = implicit_gateway_message(evm_params)?;
            tracing::debug!("query gateway parent finality message: {msg:?}");

            let (apply_ret, _) = exec_state.execute_implicit(msg)?;

            let data = apply_ret.msg_receipt.return_data.to_vec();
            let decoded = fvm_ipld_encoding::from_slice::<BytesDe>(&data)
                .map(|bz| bz.0)
                .map_err(|e| anyhow!("failed to deserialize bytes returned by FEVM: {e}"))?;
            Some(decode_parent_finality_return(decoded.as_slice())?)
        } else {
            None
        };

        Ok(finality)
    }
}

#[inline]
fn implicit_gateway_message(params: Vec<u8>) -> anyhow::Result<FvmMessage> {
    Ok(FvmMessage {
        version: 0,
        from: system::SYSTEM_ACTOR_ADDR,
        to: ipc::GATEWAY_ACTOR_ADDR,
        value: TokenAmount::zero(),
        method_num: ipc::gateway::METHOD_INVOKE_CONTRACT,
        params: RawBytes::serialize(BytesSer(&params))?,
        // we are sending a implicit message, no need to set sequence
        sequence: 0,
        gas_limit: fvm_shared::BLOCK_GAS_LIMIT,
        gas_fee_cap: TokenAmount::zero(),
        gas_premium: TokenAmount::zero(),
    })
}
