// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use async_trait::async_trait;

use fendermint_vm_message::{chain::ChainMessage, ipc::IpcMessage, signed::SignedMessage};

use crate::{
    ipc::IpcMessageCheckRet,
    signed::{SignedMessageApplyRet, SignedMessageCheckRet},
    CheckInterpreter, ExecInterpreter, GenesisInterpreter, ProposalInterpreter, QueryInterpreter,
};

// For now this is the only option, later we can expand.
pub enum ChainMessageApplyRet {
    Signed(SignedMessageApplyRet),
}

/// We only allow signed messages into the mempool.
pub enum ChainMessageCheckRet {
    Signed(SignedMessageCheckRet),
    Ipc(IpcMessageCheckRet),
}

/// Interpreter working on chain messages; in the future it will schedule
/// CID lookups to turn references into self-contained user or cross messages.
#[derive(Clone)]
pub struct ChainMessageInterpreter<S, I> {
    signed: S,
    ipc: I,
}

impl<I, C> ChainMessageInterpreter<I, C> {
    pub fn new(signed: I, ipc: C) -> Self {
        Self { signed, ipc }
    }
}

#[async_trait]
impl<I, C> ProposalInterpreter for ChainMessageInterpreter<I, C>
where
    I: Sync + Send,
    C: Sync + Send,
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
impl<I, C> ExecInterpreter for ChainMessageInterpreter<I, C>
where
    I: ExecInterpreter<Message = SignedMessage, DeliverOutput = SignedMessageApplyRet>,
    C: Sync + Send,
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
                let (state, ret) = self.signed.deliver(state, msg).await?;
                Ok((state, ChainMessageApplyRet::Signed(ret)))
            }
            ChainMessage::Ipc(_) => {
                // This only happens if a validator is malicious or we have made a programming error.
                // I expect for now that we don't run with untrusted validators, so it's okay to quit.
                todo!("#191: implement execution handling for IPC")
            }
        }
    }

    async fn begin(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::BeginOutput)> {
        // TODO #191: Return a tuple from both signed and ipc interpreters.
        self.signed.begin(state).await
    }

    async fn end(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::EndOutput)> {
        // TODO #191: Return a tuple from both signed and ipc interpreters.
        self.signed.end(state).await
    }
}

#[async_trait]
impl<I, C> CheckInterpreter for ChainMessageInterpreter<I, C>
where
    I: CheckInterpreter<Message = SignedMessage, Output = SignedMessageCheckRet>,
    C: CheckInterpreter<Message = IpcMessage, Output = IpcMessageCheckRet, State = I::State>,
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
                let (state, ret) = self.signed.check(state, msg, is_recheck).await?;

                Ok((state, ChainMessageCheckRet::Signed(ret)))
            }
            ChainMessage::Ipc(msg) => {
                let (state, ret) = self.ipc.check(state, msg, is_recheck).await?;

                Ok((state, ChainMessageCheckRet::Ipc(ret)))
            }
        }
    }
}

#[async_trait]
impl<I, C> QueryInterpreter for ChainMessageInterpreter<I, C>
where
    I: QueryInterpreter,
    C: Sync + Send,
{
    type State = I::State;
    type Query = I::Query;
    type Output = I::Output;

    async fn query(
        &self,
        state: Self::State,
        qry: Self::Query,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        self.signed.query(state, qry).await
    }
}

#[async_trait]
impl<I, C> GenesisInterpreter for ChainMessageInterpreter<I, C>
where
    I: GenesisInterpreter,
    C: Sync + Send,
{
    type State = I::State;
    type Genesis = I::Genesis;
    type Output = I::Output;

    async fn init(
        &self,
        state: Self::State,
        genesis: Self::Genesis,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        self.signed.init(state, genesis).await
    }
}
