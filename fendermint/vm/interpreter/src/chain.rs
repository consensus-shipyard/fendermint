use std::marker::PhantomData;
// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use crate::{
    fvm::FvmMessage,
    signed::{SignedMessageApplyRes, SignedMessageCheckRes, SyntheticMessage, VerifiableMessage},
    CheckInterpreter, ExecInterpreter, GenesisInterpreter, ProposalInterpreter, QueryInterpreter,
};
use anyhow::{anyhow, Context};
use async_stm::{atomically, atomically_or_err};
use async_trait::async_trait;
use fendermint_vm_actor_interface::{ipc, system};
use fendermint_vm_message::ipc::ParentFinality;
use fendermint_vm_message::{
    chain::ChainMessage,
    ipc::{BottomUpCheckpoint, CertifiedMessage, IpcMessage, SignedRelayedMessage},
};
use fendermint_vm_resolver::pool::{ResolveKey, ResolvePool};
use fendermint_vm_topdown::convert::encode_commit_parent_finality_call;
use fendermint_vm_topdown::{Error, IPCParentFinality, ParentFinalityProvider};
use fvm_ipld_encoding::RawBytes;
use fvm_shared::clock::ChainEpoch;
use fvm_shared::econ::TokenAmount;
use ipc_agent_sdk::message::ipc::ValidatorSet;
use num_traits::Zero;
use std::sync::Arc;

/// A resolution pool for bottom-up and top-down checkpoints.
pub type CheckpointPool = ResolvePool<CheckpointPoolItem>;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum CheckpointPoolItem {
    /// BottomUp checkpoints to be resolved from the originating subnet or the current one.
    BottomUp(CertifiedMessage<BottomUpCheckpoint>),
    // We can extend this to include top-down checkpoints as well, with slightly
    // different resolution semantics (resolving it from a trusted parent, and
    // awaiting finality before declaring it available).
}

impl From<&CheckpointPoolItem> for ResolveKey {
    fn from(value: &CheckpointPoolItem) -> Self {
        match value {
            CheckpointPoolItem::BottomUp(cp) => {
                (cp.message.subnet_id.clone(), cp.message.bottom_up_messages)
            }
        }
    }
}

/// A user sent a transaction which they are not allowed to do.
pub struct IllegalMessage;

// For now this is the only option, later we can expand.
pub enum ChainMessageApplyRet {
    Signed(SignedMessageApplyRes),
}

/// We only allow signed messages into the mempool.
pub type ChainMessageCheckRes = Result<SignedMessageCheckRes, IllegalMessage>;

/// Interpreter working on chain messages; in the future it will schedule
/// CID lookups to turn references into self-contained user or cross messages.
#[derive(Clone)]
pub struct ChainMessageInterpreter<I, P> {
    inner: I,
    parent_finality_provider: PhantomData<P>,
}

impl<I, P> ChainMessageInterpreter<I, P> {
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            parent_finality_provider: Default::default(),
        }
    }
}

#[async_trait]
impl<I, P> ProposalInterpreter for ChainMessageInterpreter<I, P>
where
    I: Sync + Send,
    P: ParentFinalityProvider + Send + Sync,
{
    type State = (CheckpointPool, Arc<P>);
    type Message = ChainMessage;

    /// Check whether there are any "ready" messages in the IPLD resolution mempool which can be appended to the proposal.
    ///
    /// We could also use this to select the most profitable user transactions, within the gas limit. We can also take into
    /// account the transactions which are part of top-down or bottom-up checkpoints, to stay within gas limits.
    async fn prepare(
        &self,
        state: Self::State,
        mut msgs: Vec<Self::Message>,
    ) -> anyhow::Result<Vec<Self::Message>> {
        let (pool, finality_provider) = state;

        // Prepare bottom up proposals

        // Collect resolved CIDs ready to be proposed from the pool.
        let ckpts = atomically(|| pool.collect_resolved()).await;

        // Create transactions ready to be included on the chain.
        let ckpts = ckpts.into_iter().map(|ckpt| match ckpt {
            CheckpointPoolItem::BottomUp(ckpt) => ChainMessage::Ipc(IpcMessage::BottomUpExec(ckpt)),
        });

        // Append at the end - if we run out of block space, these are going to be reproposed in the next block.
        msgs.extend(ckpts);

        // Prepare top down proposals
        match atomically_or_err::<_, Error, _>(|| finality_provider.next_proposal()).await {
            Ok(None) => {},
            Ok(Some(proposal)) => {
                msgs.push(ChainMessage::Ipc(IpcMessage::TopDownExec(ParentFinality {
                    height: proposal.height as ChainEpoch,
                    block_hash: proposal.block_hash,
                })))
            },
            // if there are errors in proposal creation, we will not crash the app, but just
            // give up proposal creation in the current block and retry in the next. there are other
            Err(e) => handle_topdown_proposal_error(e)
        }

        Ok(msgs)
    }

    /// Perform finality checks on top-down transactions and availability checks on bottom-up transactions.
    async fn process(&self, state: Self::State, msgs: Vec<Self::Message>) -> anyhow::Result<bool> {
        for msg in msgs {
            match msg {
                ChainMessage::Ipc(IpcMessage::BottomUpExec(msg)) => {
                    let item = CheckpointPoolItem::BottomUp(msg);

                    // We can just look in memory because when we start the application, we should retrieve any
                    // pending checkpoints (relayed but not executed) from the ledger, so they should be there.
                    // We don't have to validate the checkpoint here, because
                    // 1) we validated it when it was relayed, and
                    // 2) if a validator proposes something invalid, we can make them pay during execution.
                    let is_resolved = atomically(|| match state.0.get_status(&item)? {
                        None => Ok(false),
                        Some(status) => status.is_resolved(),
                    })
                    .await;

                    if !is_resolved {
                        return Ok(false);
                    }
                }
                ChainMessage::Ipc(IpcMessage::TopDownExec(ParentFinality {
                    height,
                    block_hash,
                })) => {
                    let prop = IPCParentFinality {
                        height: height as u64,
                        block_hash,
                    };
                    if atomically_or_err(|| state.1.check_proposal(&prop))
                        .await
                        .is_err()
                    {
                        return Ok(false);
                    }
                }
                _ => {}
            };
        }
        Ok(true)
    }
}

#[async_trait]
impl<I, P> ExecInterpreter for ChainMessageInterpreter<I, P>
where
    I: ExecInterpreter<Message = VerifiableMessage, DeliverOutput = SignedMessageApplyRes>,
    P: ParentFinalityProvider + Send + Sync,
{
    // The state consists of the resolver pool, which this interpreter needs, and the rest of the
    // state which the inner interpreter uses. This is a technical solution because the pool doesn't
    // fit with the state we use for execution messages further down the stack, which depend on block
    // height and are used in queries as well.
    type State = (CheckpointPool, Arc<P>, I::State);
    type Message = ChainMessage;
    type BeginOutput = I::BeginOutput;
    type DeliverOutput = ChainMessageApplyRet;
    type EndOutput = I::EndOutput;

    async fn deliver(
        &self,
        (pool, provider, state): Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::DeliverOutput)> {
        match msg {
            ChainMessage::Signed(msg) => {
                let (state, ret) = self
                    .inner
                    .deliver(state, VerifiableMessage::Signed(msg))
                    .await?;
                Ok(((pool, provider, state), ChainMessageApplyRet::Signed(ret)))
            }
            ChainMessage::Ipc(msg) => match msg {
                IpcMessage::BottomUpResolve(msg) => {
                    let smsg = relayed_bottom_up_ckpt_to_fvm(&msg)
                        .context("failed to syntesize FVM message")?;

                    // Let the FVM validate the checkpoint quorum certificate and take note of the relayer for rewards.
                    let (state, ret) = self
                        .inner
                        .deliver(state, VerifiableMessage::Synthetic(smsg))
                        .await?;

                    // If successful, add the CID to the background resolution pool.
                    let is_success = match ret {
                        Ok(ref ret) => ret.fvm.apply_ret.msg_receipt.exit_code.is_success(),
                        Err(_) => false,
                    };

                    if is_success {
                        atomically(|| {
                            pool.add(CheckpointPoolItem::BottomUp(msg.message.message.clone()))
                        })
                        .await;
                    }

                    // We can use the same result type for now, it's isomorphic.
                    Ok(((pool, provider, state), ChainMessageApplyRet::Signed(ret)))
                }
                IpcMessage::BottomUpExec(_) => {
                    todo!("#197: implement BottomUp checkpoint execution")
                }
                IpcMessage::TopDownExec(p) => {
                    let validator_set =
                        atomically_or_err::<_, fendermint_vm_topdown::Error, _>(|| {
                            provider.validator_set(p.height as u64)
                        })
                        .await?
                        .ok_or_else(|| {
                            anyhow!("cannot find validator set for block: {}", p.height)
                        })?;

                    let finality = IPCParentFinality {
                        height: p.height as u64,
                        block_hash: p.block_hash,
                    };
                    let msg = parent_finality_to_fvm(finality.clone(), validator_set)?;
                    let (state, ret) = self
                        .inner
                        .deliver(state, VerifiableMessage::NotVerify(msg))
                        .await?;

                    atomically_or_err::<_, fendermint_vm_topdown::Error, _>(|| {
                        provider.set_new_finality(finality.clone())
                    })
                    .await?;

                    // TODO: execute top down messages,
                    // TODO: see https://github.com/consensus-shipyard/fendermint/issues/241

                    Ok(((pool, provider, state), ChainMessageApplyRet::Signed(ret)))
                }
            },
        }
    }

    async fn begin(
        &self,
        (pool, provider, state): Self::State,
    ) -> anyhow::Result<(Self::State, Self::BeginOutput)> {
        let (state, out) = self.inner.begin(state).await?;
        Ok(((pool, provider, state), out))
    }

    async fn end(
        &self,
        (pool, provider, state): Self::State,
    ) -> anyhow::Result<(Self::State, Self::EndOutput)> {
        let (state, out) = self.inner.end(state).await?;
        Ok(((pool, provider, state), out))
    }
}

#[async_trait]
impl<I, P> CheckInterpreter for ChainMessageInterpreter<I, P>
where
    I: CheckInterpreter<Message = VerifiableMessage, Output = SignedMessageCheckRes>,
    P: Send + Sync,
{
    type State = I::State;
    type Message = ChainMessage;
    type Output = ChainMessageCheckRes;

    async fn check(
        &self,
        state: Self::State,
        msg: Self::Message,
        is_recheck: bool,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        match msg {
            ChainMessage::Signed(msg) => {
                let (state, ret) = self
                    .inner
                    .check(state, VerifiableMessage::Signed(msg), is_recheck)
                    .await?;

                Ok((state, Ok(ret)))
            }
            ChainMessage::Ipc(msg) => {
                match msg {
                    IpcMessage::BottomUpResolve(msg) => {
                        let msg = relayed_bottom_up_ckpt_to_fvm(&msg)
                            .context("failed to syntesize FVM message")?;

                        let (state, ret) = self
                            .inner
                            .check(state, VerifiableMessage::Synthetic(msg), is_recheck)
                            .await?;

                        Ok((state, Ok(ret)))
                    }
                    IpcMessage::TopDownExec(_) | IpcMessage::BottomUpExec(_) => {
                        // Users cannot send these messages, only validators can propose them in blocks.
                        Ok((state, Err(IllegalMessage)))
                    }
                }
            }
        }
    }
}

#[async_trait]
impl<I, P> QueryInterpreter for ChainMessageInterpreter<I, P>
where
    I: QueryInterpreter,
    P: Send + Sync,
{
    type State = I::State;
    type Query = I::Query;
    type Output = I::Output;

    async fn query(
        &self,
        state: Self::State,
        qry: Self::Query,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        self.inner.query(state, qry).await
    }
}

#[async_trait]
impl<I, P> GenesisInterpreter for ChainMessageInterpreter<I, P>
where
    I: GenesisInterpreter,
    P: Send + Sync,
{
    type State = I::State;
    type Genesis = I::Genesis;
    type Output = I::Output;

    async fn init(
        &self,
        state: Self::State,
        genesis: Self::Genesis,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        self.inner.init(state, genesis).await
    }
}

/// Convert a signed relayed bottom-up checkpoint to a syntetic message we can send to the FVM.
///
/// By mapping to an FVM message we invoke the right contract to validate the checkpoint,
/// and automatically charge the relayer gas for the execution of the check, but not the
/// execution of the cross-messages, which aren't part of the payload.
fn relayed_bottom_up_ckpt_to_fvm(
    relayed: &SignedRelayedMessage<CertifiedMessage<BottomUpCheckpoint>>,
) -> anyhow::Result<SyntheticMessage> {
    // TODO #192: Convert the checkpoint to what the actor expects.
    let params = RawBytes::default();

    let msg = FvmMessage {
        version: 0,
        from: relayed.message.relayer,
        to: ipc::GATEWAY_ACTOR_ADDR,
        sequence: relayed.message.sequence,
        value: TokenAmount::zero(),
        method_num: ipc::gateway::METHOD_INVOKE_CONTRACT,
        params,
        gas_limit: relayed.message.gas_limit,
        gas_fee_cap: relayed.message.gas_fee_cap.clone(),
        gas_premium: relayed.message.gas_premium.clone(),
    };

    let msg = SyntheticMessage::new(msg, &relayed.message, relayed.signature.clone())
        .context("failed to create syntetic message")?;

    Ok(msg)
}

/// Convert a parent finality to fvm message
fn parent_finality_to_fvm(
    finality: IPCParentFinality,
    validator_set: ValidatorSet,
) -> anyhow::Result<FvmMessage> {
    let params = RawBytes::new(encode_commit_parent_finality_call(finality, validator_set)?);
    let msg = FvmMessage {
        version: 0,
        from: system::SYSTEM_ACTOR_ADDR,
        to: ipc::GATEWAY_ACTOR_ADDR,
        value: TokenAmount::zero(),
        method_num: ipc::gateway::METHOD_INVOKE_CONTRACT,
        params,
        // we are sending a implicit message, no need to set sequence
        sequence: 0,
        gas_limit: fvm_shared::BLOCK_GAS_LIMIT,
        gas_fee_cap: TokenAmount::zero(),
        gas_premium: TokenAmount::zero(),
    };

    Ok(msg)
}

/// Handles the error thrown in proposing top down parent finality.
fn handle_topdown_proposal_error(err: Error) {
    match err {
        Error::HeightNotReady | Error::HeightThresholdNotReached => {
            tracing::debug!("top down proposal error: {err:?}");
        },
        Error::HeightNotFoundInCache(height) => {
            tracing::
        }
        _ => {}
    }
}