use std::marker::PhantomData;

use anyhow::anyhow;
use async_trait::async_trait;

use fendermint_vm_message::{SignedMessage, SignedMessageError};
use fvm::{
    call_manager::DefaultCallManager,
    executor::{ApplyRet, DefaultExecutor, Executor},
    machine::DefaultMachine,
    DefaultKernel,
};
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::message::Message as FvmMessage;

use crate::{externs::FendermintExterns, Interpreter};

pub struct FvmState<DB>
where
    DB: Blockstore + 'static,
{
    executor:
        DefaultExecutor<DefaultKernel<DefaultCallManager<DefaultMachine<DB, FendermintExterns>>>>,
}

/// Interpreter working on already verified unsigned messages.
pub struct MessageInterpreter<DB> {
    _phantom_db: PhantomData<DB>,
}

#[async_trait]
impl<DB> Interpreter for MessageInterpreter<DB>
where
    DB: Blockstore + 'static + Send + Sync,
{
    type Message = FvmMessage;
    type State = FvmState<DB>;
    type Output = ApplyRet;

    async fn exec_msg(
        &self,
        mut state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        let raw_length = fvm_ipld_encoding::to_vec(&msg).map(|bz| bz.len())?;
        let ret =
            state
                .executor
                .execute_message(msg, fvm::executor::ApplyKind::Explicit, raw_length)?;
        Ok((state, ret))
    }
}

/// Interpreter working on signed messages, validating their signature before sending
/// the unsigned parts on for execution.
pub struct SignedMessageInterpreter<MI> {
    message_interpreter: MI,
}

pub enum SignedMesssageApplyRet {
    InvalidSignature(String),
    Applied(ApplyRet),
}

#[async_trait]
impl<MI> Interpreter for SignedMessageInterpreter<MI>
where
    MI: Interpreter<Message = FvmMessage, Output = ApplyRet>,
{
    type Message = SignedMessage;
    type Output = SignedMesssageApplyRet;
    type State = MI::State;

    async fn exec_msg(
        &self,
        state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        match msg.verify() {
            Err(SignedMessageError::Ipld(e)) => Err(anyhow!(e)),
            Err(SignedMessageError::InvalidSignature(s)) => {
                Ok((state, SignedMesssageApplyRet::InvalidSignature(s)))
            }
            Ok(()) => {
                let (state, ret) = self
                    .message_interpreter
                    .exec_msg(state, msg.message)
                    .await?;

                Ok((state, SignedMesssageApplyRet::Applied(ret)))
            }
        }
    }
}
