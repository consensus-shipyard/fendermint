use anyhow::anyhow;
use async_trait::async_trait;

use fendermint_vm_message::signed::{SignedMessage, SignedMessageError};
use fvm::executor::ApplyRet;

use crate::{fvm::FvmMessage, Interpreter};

/// Interpreter working on signed messages, validating their signature before sending
/// the unsigned parts on for execution.
pub struct SignedMessageInterpreter<I> {
    inner: I,
}

pub enum SignedMesssageApplyRet {
    InvalidSignature(String),
    Applied(ApplyRet),
}

#[async_trait]
impl<I> Interpreter for SignedMessageInterpreter<I>
where
    I: Interpreter<Message = FvmMessage, Output = ApplyRet>,
{
    type Message = SignedMessage;
    type Output = SignedMesssageApplyRet;
    type State = I::State;

    async fn exec(
        &self,
        state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        match msg.verify() {
            Err(SignedMessageError::Ipld(e)) => Err(anyhow!(e)),
            Err(SignedMessageError::InvalidSignature(s)) => {
                // TODO: We can penalize the validator for including an invalid signature.
                Ok((state, SignedMesssageApplyRet::InvalidSignature(s)))
            }
            Ok(()) => {
                let (state, ret) = self.inner.exec(state, msg.message).await?;

                Ok((state, SignedMesssageApplyRet::Applied(ret)))
            }
        }
    }
}
