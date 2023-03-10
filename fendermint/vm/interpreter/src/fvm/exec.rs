// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use async_trait::async_trait;

use fendermint_vm_actor_interface::{cron, system};
use fvm::executor::ApplyRet;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::BLOCK_GAS_LIMIT;

use crate::Interpreter;

use super::{FvmMessage, FvmMessageInterpreter, FvmState};

/// The return value extended with some things from the message that
/// might not be available to the caller, because of the message lookups
/// and transformations that happen along the way, e.g. where we need
/// a field, we might just have a CID.
pub struct FvmApplyRet {
    pub apply_ret: ApplyRet,
    pub gas_limit: u64,
}

impl<DB> Default for FvmMessageInterpreter<DB> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<DB> Interpreter for FvmMessageInterpreter<DB>
where
    DB: Blockstore + 'static + Send + Sync,
{
    type State = FvmState<DB>;
    type Message = FvmMessage;
    type BeginOutput = FvmApplyRet;
    type DeliverOutput = FvmApplyRet;
    type EndOutput = ();

    async fn begin(
        &self,
        mut state: Self::State,
    ) -> anyhow::Result<(Self::State, Self::BeginOutput)> {
        // Block height (FVM epoch) as sequence is intentional
        let height = state.block_height();
        // Arbitrarily large gas limit for cron (matching how Forest does it, which matches Lotus).
        // XXX: Our blocks are not necessarily expected to be 30 seconds apart, so the gas limit might be wrong.
        let gas_limit = BLOCK_GAS_LIMIT * 10000;
        // Cron.
        let msg = FvmMessage {
            from: system::SYSTEM_ACTOR_ADDR,
            to: cron::CRON_ACTOR_ADDR,
            sequence: height as u64,
            gas_limit,
            method_num: cron::Method::EpochTick as u64,
            params: Default::default(),
            value: Default::default(),
            version: Default::default(),
            gas_fee_cap: Default::default(),
            gas_premium: Default::default(),
        };

        let apply_ret = state.execute_implicit(msg)?;

        // Failing cron would be fatal.
        if let Some(err) = apply_ret.failure_info {
            anyhow::bail!("failed to apply block cron message: {}", err);
        }

        let ret = FvmApplyRet {
            apply_ret,
            gas_limit,
        };

        Ok((state, ret))
    }

    async fn deliver(
        &self,
        mut state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::DeliverOutput)> {
        let gas_limit = msg.gas_limit;

        let apply_ret = state.execute_explicit(msg)?;

        let ret = FvmApplyRet {
            apply_ret,
            gas_limit,
        };

        Ok((state, ret))
    }

    async fn end(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::EndOutput)> {
        // TODO: Epoch transitions for checkpointing.
        Ok((state, ()))
    }
}
