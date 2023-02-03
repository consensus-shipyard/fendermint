use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cid::Cid;
use fendermint_abci::Application;
use fendermint_vm_interpreter::chain::ChainMessageApplyRet;
use fendermint_vm_interpreter::fvm::FvmState;
use fendermint_vm_interpreter::{Interpreter, Timestamp};
use fendermint_vm_message::chain::ChainMessage;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::econ::TokenAmount;
use fvm_shared::version::NetworkVersion;
use tendermint::abci::{request, response};

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct State {
    block_height: u64,
    state_root: Cid,
    network_version: NetworkVersion,
    base_fee: TokenAmount,
    circ_supply: TokenAmount,
}

/// Handle ABCI requests.
pub struct FendermintApp<DB, I>
where
    DB: Blockstore + 'static,
{
    db: Arc<DB>,
    interpreter: Arc<I>,
    /// State accumulating changes during block execution.
    exec_state: Arc<Mutex<Option<FvmState<DB>>>>,
}

impl<DB, I> FendermintApp<DB, I>
where
    DB: Blockstore + 'static,
{
    pub fn new(db: Arc<DB>, interpreter: I) -> Self {
        Self {
            db,
            interpreter: Arc::new(interpreter),
            exec_state: Arc::new(Mutex::new(None)),
        }
    }
}

impl<DB, I> FendermintApp<DB, I>
where
    DB: Blockstore + 'static,
{
    /// Get the last committed state.
    fn committed_state(&self) -> State {
        todo!("retrieve state from the DB")
    }

    /// Put the execution state during block execution. Has to be empty.
    fn put_exec_state(&self, state: FvmState<DB>) {
        let mut guard = self.exec_state.lock().expect("mutex poisoned");
        assert!(guard.is_some(), "exec state not empty");
        *guard = Some(state);
    }

    /// Take the execution state during block execution. Has to be non-empty.
    fn take_exec_state(&self) -> FvmState<DB> {
        let mut guard = self.exec_state.lock().expect("mutex poisoned");
        guard.take().expect("exec state empty")
    }
}

// NOTE: The `Application` interface doesn't allow failures at the moment. The protobuf
// of `Response` actually has an `Exception` type, so in theory we could use that, and
// Tendermint would break up the connection. However, before the response could reach it,
// the `tower-abci` library would throw an exception because when it tried to convert
// a `Response::Exception` into a `ConensusResponse` for example.
#[async_trait]
impl<DB, I> Application for FendermintApp<DB, I>
where
    DB: Blockstore + Clone + Send + Sync + 'static,
    I: Interpreter<
        State = FvmState<DB>,
        Message = ChainMessage,
        BeginOutput = (),
        DeliverOutput = ChainMessageApplyRet,
        EndOutput = (),
    >,
{
    /// Provide information about the ABCI application.
    async fn info(&self, _request: request::Info) -> response::Info {
        let state = self.committed_state();
        let height =
            tendermint::block::Height::try_from(state.block_height).expect("height too big");
        let app_hash = tendermint::hash::AppHash::try_from(state.state_root.to_bytes())
            .expect("hash can be wrapped");
        response::Info {
            data: "fendermint".to_string(),
            version: VERSION.to_owned(),
            app_version: 1,
            last_block_height: height,
            last_block_app_hash: app_hash,
        }
    }

    /// Called once upon genesis.
    async fn init_chain(&self, _request: request::InitChain) -> response::InitChain {
        Default::default()
    }

    /// Query the application for data at the current or past height.
    async fn query(&self, _request: request::Query) -> response::Query {
        todo!("make a query interpreter")
    }

    /// Check the given transaction before putting it into the local mempool.
    async fn check_tx(&self, _request: request::CheckTx) -> response::CheckTx {
        todo!("make an interpreter for checks, on a projected state")
    }

    /// Signals the beginning of a new block, prior to any `DeliverTx` calls.
    async fn begin_block(&self, request: request::BeginBlock) -> response::BeginBlock {
        let state = self.committed_state();
        let height = request.header.height.into();
        let timestamp = Timestamp(
            request
                .header
                .time
                .unix_timestamp()
                .try_into()
                .expect("negative timestamp"),
        );
        let db = self.db.as_ref().to_owned();

        let state = FvmState::new(
            db,
            height,
            timestamp,
            state.network_version,
            state.state_root,
            state.base_fee,
            state.circ_supply,
        )
        .expect("error creating new state");

        let (state, ()) = self.interpreter.begin(state).await.expect("begin failed");

        self.put_exec_state(state);

        response::BeginBlock { events: Vec::new() }
    }

    /// Apply a transaction to the application's state.
    async fn deliver_tx(&self, request: request::DeliverTx) -> response::DeliverTx {
        todo!()
    }

    /// Signals the end of a block.
    async fn end_block(&self, _request: request::EndBlock) -> response::EndBlock {
        todo!()
    }

    /// Commit the current state at the current height.
    async fn commit(&self) -> response::Commit {
        todo!()
    }
}
