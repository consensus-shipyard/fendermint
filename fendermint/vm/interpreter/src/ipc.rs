// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use async_trait::async_trait;
use fendermint_vm_core::chainid::HasChainID;
use fendermint_vm_message::ipc::IpcMessage;

use crate::{
    fvm::{FvmCheckRet, FvmMessage},
    CheckInterpreter,
};

#[derive(Debug, thiserror::Error)]
pub enum IpcMessageError {
    #[error("the user is not supposed to send this kind of message to the mempool")]
    Illegal,
    #[error("invalid relayer signature: {0}")]
    InvalidRelayerSignature(String),
}

pub type IpcMessageCheckRet = Result<FvmCheckRet, IpcMessageError>;

/// Interpreter working on IPC messages.
///
/// It performs operations such as checking relayer signatures,
/// while delegating FVM specific checks to an inner interpreter.
#[derive(Clone)]
pub struct IpcMessageInterpreter<I> {
    _inner: I,
}

impl<I> IpcMessageInterpreter<I> {
    pub fn new(inner: I) -> Self {
        Self { _inner: inner }
    }
}

#[async_trait]
impl<I, S> CheckInterpreter for IpcMessageInterpreter<I>
where
    I: CheckInterpreter<Message = FvmMessage, Output = FvmCheckRet, State = S>,
    S: HasChainID + Send + 'static,
{
    type State = I::State;
    type Message = IpcMessage;
    type Output = IpcMessageCheckRet;

    async fn check(
        &self,
        state: Self::State,
        msg: Self::Message,
        _is_recheck: bool,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        match msg {
            IpcMessage::BottomUpResolve(_msg) => {
                todo!("check signature, then pass on to the innter interpreter")
            }
            IpcMessage::TopDown | IpcMessage::BottomUpExec(_) => {
                // Users cannot send these messages, only validators can propose them in blocks.
                Ok((state, Err(IpcMessageError::Illegal)))
            }
        }
    }
}
