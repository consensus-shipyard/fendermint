use std::marker::PhantomData;

use async_trait::async_trait;

use cid::Cid;
use fvm::{
    call_manager::DefaultCallManager,
    engine::{EngineConfig, EnginePool},
    executor::{ApplyRet, DefaultExecutor, Executor},
    machine::{DefaultMachine, NetworkConfig},
    DefaultKernel,
};
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::{clock::ChainEpoch, econ::TokenAmount, version::NetworkVersion};

use crate::{externs::FendermintExterns, Interpreter, Timestamp};

pub type FvmMessage = fvm_shared::message::Message;
pub type FvmApplyRet = ApplyRet;

/// A state we create for the execution of all the messages in a block.
pub struct FvmState<DB>
where
    DB: Blockstore + 'static,
{
    executor:
        DefaultExecutor<DefaultKernel<DefaultCallManager<DefaultMachine<DB, FendermintExterns>>>>,
}

impl<DB> FvmState<DB>
where
    DB: Blockstore + 'static,
{
    pub fn new(
        blockstore: DB,
        block_height: ChainEpoch,
        block_timestamp: Timestamp,
        network_version: NetworkVersion,
        initial_state: Cid,
        base_fee: TokenAmount,
        circ_supply: TokenAmount,
    ) -> anyhow::Result<Self> {
        let nc = NetworkConfig::new(network_version);

        // TODO: Configure:
        // * circ_supply; by default it's for Filecoin
        // * base_fee; by default it's zero
        let mut mc = nc.for_epoch(block_height, block_timestamp.0, initial_state);
        mc.set_base_fee(base_fee);
        mc.set_circulating_supply(circ_supply);

        let ec = EngineConfig::from(&nc);
        let engine = EnginePool::new_default(ec)?;
        let machine = DefaultMachine::new(&mc, blockstore, FendermintExterns)?;
        let executor = DefaultExecutor::new(engine, machine)?;

        Ok(Self { executor })
    }

    /// Commit the state. It must not fail, but we're returning a result so that error
    /// handling can be done in the application root.
    ///
    /// For now this is not part of the `Interpreter` because it's not clear what atomic
    /// semantics we can hope to provide if the middlewares call each other: did it go
    /// all the way down, or did it stop somewhere? Easier to have one commit of the state
    /// as a whole.
    pub fn commit(mut self) -> anyhow::Result<Cid> {
        self.executor.flush()
    }
}

/// Interpreter working on already verified unsigned messages.
pub struct FvmMessageInterpreter<DB> {
    _phantom_db: PhantomData<DB>,
}

impl<DB> FvmMessageInterpreter<DB> {
    pub fn new() -> Self {
        Self {
            _phantom_db: PhantomData,
        }
    }
}

#[async_trait]
impl<DB> Interpreter for FvmMessageInterpreter<DB>
where
    DB: Blockstore + 'static + Send + Sync,
{
    type State = FvmState<DB>;
    type Message = FvmMessage;
    type BeginOutput = ();
    type DeliverOutput = FvmApplyRet;
    type EndOutput = ();

    async fn begin(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::BeginOutput)> {
        // TODO: Cron.
        Ok((state, ()))
    }

    async fn deliver(
        &self,
        mut state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::DeliverOutput)> {
        let raw_length = fvm_ipld_encoding::to_vec(&msg).map(|bz| bz.len())?;
        let ret =
            state
                .executor
                .execute_message(msg, fvm::executor::ApplyKind::Explicit, raw_length)?;
        Ok((state, ret))
    }

    async fn end(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::EndOutput)> {
        // TODO: Epoch transitions for checkpointing.
        Ok((state, ()))
    }
}
