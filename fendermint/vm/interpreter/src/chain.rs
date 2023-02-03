use async_trait::async_trait;

use fendermint_vm_message::{chain::ChainMessage, signed::SignedMessage};

use crate::{signed::SignedMesssageApplyRet, Interpreter};

/// Interpreter working on chain messages; in the future it will schedule
/// CID lookups to turn references into self-contained user or cross messages.
pub struct ChainMessageInterpreter<I> {
    inner: I,
}

pub enum ChainMessageApplyRet {
    Signed(SignedMesssageApplyRet),
}

#[async_trait]
impl<I> Interpreter for ChainMessageInterpreter<I>
where
    I: Interpreter<Message = SignedMessage, Output = SignedMesssageApplyRet>,
{
    type Message = ChainMessage;
    type Output = ChainMessageApplyRet;
    type State = I::State;

    async fn exec(
        &self,
        state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        match msg {
            ChainMessage::Signed(msg) => {
                let (state, ret) = self.inner.exec(state, msg).await?;
                Ok((state, ChainMessageApplyRet::Signed(ret)))
            }
        }
    }
}
