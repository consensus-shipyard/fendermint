// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use crate::{
    fvm::FvmMessage,
    signed::{SignedMessageApplyRet, SignedMessageCheckRet, SyntheticMessage, VerifiableMessage},
    CheckInterpreter, ExecInterpreter, GenesisInterpreter, ProposalInterpreter, QueryInterpreter,
};
use anyhow::Context;
use async_trait::async_trait;
use fendermint_vm_actor_interface::ipc;
use fendermint_vm_message::{
    chain::ChainMessage,
    ipc::{BottomUpCheckpoint, CertifiedMessage, IpcMessage, SignedRelayedMessage},
};
use fvm_ipld_encoding::RawBytes;
use fvm_shared::econ::TokenAmount;
use num_traits::Zero;

/// A user sent a transaction which they are not allowed to do.
pub struct IllegalMessage;

// For now this is the only option, later we can expand.
pub enum ChainMessageApplyRet {
    Signed(SignedMessageApplyRet),
}

/// We only allow signed messages into the mempool.
pub type ChainMessageCheckRet = Result<SignedMessageCheckRet, IllegalMessage>;

/// Interpreter working on chain messages; in the future it will schedule
/// CID lookups to turn references into self-contained user or cross messages.
#[derive(Clone)]
pub struct ChainMessageInterpreter<I> {
    inner: I,
}

impl<I> ChainMessageInterpreter<I> {
    pub fn new(inner: I) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<I> ProposalInterpreter for ChainMessageInterpreter<I>
where
    I: Sync + Send,
{
    // TODO: The state can include the IPLD Resolver mempool, for example by using STM
    // to implement a shared memory space.
    type State = ();
    type Message = ChainMessage;

    /// Check whether there are any "ready" messages in the IPLD resolution mempool which can be appended to the proposal.
    ///
    /// We could also use this to select the most profitable user transactions, within the gas limit. We can also take into
    /// account the transactions which are part of top-down or bottom-up checkpoints, to stay within gas limits.
    async fn prepare(
        &self,
        _state: Self::State,
        msgs: Vec<Self::Message>,
    ) -> anyhow::Result<Vec<Self::Message>> {
        // For now this is just a placeholder.
        Ok(msgs)
    }

    /// Perform finality checks on top-down transactions and availability checks on bottom-up transactions.
    async fn process(
        &self,
        _state: Self::State,
        _msgs: Vec<Self::Message>,
    ) -> anyhow::Result<bool> {
        // For now this is just a placeholder.
        Ok(true)
    }
}

#[async_trait]
impl<I> ExecInterpreter for ChainMessageInterpreter<I>
where
    I: ExecInterpreter<Message = VerifiableMessage, DeliverOutput = SignedMessageApplyRet>,
{
    type State = I::State;
    type Message = ChainMessage;
    type BeginOutput = I::BeginOutput;
    type DeliverOutput = ChainMessageApplyRet;
    type EndOutput = I::EndOutput;

    async fn deliver(
        &self,
        state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::DeliverOutput)> {
        match msg {
            ChainMessage::Signed(msg) => {
                let (state, ret) = self
                    .inner
                    .deliver(state, VerifiableMessage::Signed(msg))
                    .await?;
                Ok((state, ChainMessageApplyRet::Signed(ret)))
            }
            ChainMessage::Ipc(msg) => match msg {
                IpcMessage::BottomUpResolve(msg) => {
                    let msg = relayed_bottom_up_ckpt_to_fvm(&msg)
                        .context("failed to syntesize FVM message")?;

                    // Let the FVM validate the checkpoint quorum certificate and take not of the relayer for rewards.
                    let (state, ret) = self
                        .inner
                        .deliver(state, VerifiableMessage::Synthetic(msg))
                        .await?;

                    // TODO: if successful, add the CID to the background resolution pool

                    // We can use the same result type for now, it's isomorphic.
                    Ok((state, ChainMessageApplyRet::Signed(ret)))
                }
                IpcMessage::BottomUpExec(_) => {
                    todo!("#197: implement BottomUp checkpoint execution")
                }
                IpcMessage::TopDown => {
                    todo!("implement TopDown handling; this is just a placeholder")
                }
            },
        }
    }

    async fn begin(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::BeginOutput)> {
        self.inner.begin(state).await
    }

    async fn end(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::EndOutput)> {
        self.inner.end(state).await
    }
}

#[async_trait]
impl<I> CheckInterpreter for ChainMessageInterpreter<I>
where
    I: CheckInterpreter<Message = VerifiableMessage, Output = SignedMessageCheckRet>,
{
    type State = I::State;
    type Message = ChainMessage;
    type Output = ChainMessageCheckRet;

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
                    IpcMessage::TopDown | IpcMessage::BottomUpExec(_) => {
                        // Users cannot send these messages, only validators can propose them in blocks.
                        Ok((state, Err(IllegalMessage)))
                    }
                }
            }
        }
    }
}

#[async_trait]
impl<I> QueryInterpreter for ChainMessageInterpreter<I>
where
    I: QueryInterpreter,
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
impl<I> GenesisInterpreter for ChainMessageInterpreter<I>
where
    I: GenesisInterpreter,
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
