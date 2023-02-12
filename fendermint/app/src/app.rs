// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use std::future::Future;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cid::Cid;
use fendermint_abci::Application;
use fendermint_storage::{Codec, Encode, KVRead, KVReadable, KVStore, KVWritable, KVWrite};
use fendermint_vm_interpreter::bytes::{BytesMessageApplyRet, BytesMessageCheckRet};
use fendermint_vm_interpreter::chain::{ChainMessageApplyRet, IllegalMessage};
use fendermint_vm_interpreter::fvm::{FvmApplyRet, FvmCheckRet, FvmCheckState, FvmState};
use fendermint_vm_interpreter::signed::InvalidSignature;
use fendermint_vm_interpreter::{CheckInterpreter, Interpreter, Timestamp};
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::event::StampedEvent;
use fvm_shared::version::NetworkVersion;
use serde::{Deserialize, Serialize};
use tendermint::abci::{request, response, Code, Event};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
#[repr(u8)]
pub enum AppStoreKey {
    State,
}

// TODO: What range should we use for our own error codes? Should we shift FVM errors?
#[repr(u32)]
enum AppError {
    /// Failed to deserialize the transaction.
    InvalidEncoding = 51,
    /// Failed to validate the user signature.
    InvalidSignature = 52,
    /// User sent a message they should not construct.
    IllegalMessage = 53,
}

#[derive(Serialize, Deserialize)]
pub struct AppState {
    block_height: u64,
    state_root: Cid, // TODO: Use TCid
    network_version: NetworkVersion,
    base_fee: TokenAmount,
    circ_supply: TokenAmount,
}

struct AppStore<DB, S>
where
    DB: Blockstore + 'static,
    S: KVStore,
{
    db: DB,
    namespace: S::Namespace,
}

/// Handle ABCI requests.
#[derive(Clone)]
pub struct App<DB, S, I>
where
    DB: Blockstore + 'static,
    S: KVStore,
{
    store: Arc<AppStore<DB, S>>,
    /// Interpreter for block lifecycle events.
    interpreter: Arc<I>,
    /// State accumulating changes during block execution.
    exec_state: Arc<Mutex<Option<FvmState<DB>>>>,
    /// Projected partial state accumulating during transaction checks.
    check_state: Arc<tokio::sync::Mutex<Option<FvmCheckState<DB>>>>,
}

impl<DB, S, I> App<DB, S, I>
where
    S: KVStore + Codec<AppState> + Encode<AppStoreKey>,
    DB: Blockstore + KVWritable<S> + KVReadable<S> + Clone + 'static,
{
    pub fn new(db: DB, namespace: S::Namespace, interpreter: I) -> Self {
        let store = AppStore { db, namespace };
        Self {
            store: Arc::new(store),
            interpreter: Arc::new(interpreter),
            exec_state: Arc::new(Mutex::new(None)),
            check_state: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

impl<DB, S> AppStore<DB, S>
where
    S: KVStore + Codec<AppState> + Encode<AppStoreKey>,
    DB: Blockstore + KVWritable<S> + KVReadable<S> + 'static,
{
    /// Get the last committed state.
    fn committed_state(&self) -> AppState {
        let tx = self.db.read();
        tx.get(&self.namespace, &AppStoreKey::State)
            .expect("get failed")
            .expect("app state not found") // TODO: Init during setup.
    }

    /// Set the last committed state.
    fn set_committed_state(&self, state: AppState) {
        self.db
            .with_write(|tx| tx.put(&self.namespace, &AppStoreKey::State, &state))
            .expect("commit failed");
    }
}

impl<DB, S, I> App<DB, S, I>
where
    DB: Blockstore + 'static,
    S: KVStore,
{
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

    /// Take the execution state, update it, put it back, return the output.
    async fn modify_exec_state<T, F, R>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(FvmState<DB>) -> R,
        R: Future<Output = anyhow::Result<(FvmState<DB>, T)>>,
    {
        let state = self.take_exec_state();
        let (state, ret) = f(state).await?;
        self.put_exec_state(state);
        Ok(ret)
    }
}

// NOTE: The `Application` interface doesn't allow failures at the moment. The protobuf
// of `Response` actually has an `Exception` type, so in theory we could use that, and
// Tendermint would break up the connection. However, before the response could reach it,
// the `tower-abci` library would throw an exception when it tried to convert a
// `Response::Exception` into a `ConensusResponse` for example.
#[async_trait]
impl<DB, S, I> Application for App<DB, S, I>
where
    S: KVStore + Codec<AppState> + Encode<AppStoreKey>,
    S::Namespace: Sync + Send,
    DB: Blockstore + KVWritable<S> + KVReadable<S> + Clone + Send + Sync + 'static,
    I: Interpreter<
        State = FvmState<DB>,
        Message = Vec<u8>,
        BeginOutput = FvmApplyRet,
        DeliverOutput = BytesMessageApplyRet,
        EndOutput = (),
    >,
    I: CheckInterpreter<
        State = FvmCheckState<DB>,
        Message = Vec<u8>,
        Output = BytesMessageCheckRet,
    >,
{
    /// Provide information about the ABCI application.
    async fn info(&self, _request: request::Info) -> response::Info {
        let state = self.store.committed_state();
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
    async fn check_tx(&self, request: request::CheckTx) -> response::CheckTx {
        // Keep the guard through the check, so there can be only one at a time.
        let mut guard = self.check_state.lock().await;

        let state = guard.take().unwrap_or_else(|| {
            let state = self.store.committed_state();
            FvmCheckState::new(self.store.db.clone(), state.state_root)
                .expect("error creating check state")
        });

        // TODO: We can make use of `request.kind` to skip signature checks on repeated calls.
        let (state, result) = self
            .interpreter
            .check(state, request.tx.to_vec())
            .await
            .expect("error running check");

        // Update the check state.
        *guard = Some(state);

        match result {
            Err(e) => invalid_check_tx(AppError::InvalidEncoding, e.description),
            Ok(result) => match result {
                Err(IllegalMessage) => invalid_check_tx(AppError::IllegalMessage, "".to_owned()),
                Ok(result) => match result {
                    Err(InvalidSignature(d)) => invalid_check_tx(AppError::InvalidSignature, d),
                    Ok(ret) => to_check_tx(ret),
                },
            },
        }
    }

    /// Signals the beginning of a new block, prior to any `DeliverTx` calls.
    async fn begin_block(&self, request: request::BeginBlock) -> response::BeginBlock {
        let state = self.store.committed_state();
        let height = request.header.height.into();
        let timestamp = Timestamp(
            request
                .header
                .time
                .unix_timestamp()
                .try_into()
                .expect("negative timestamp"),
        );
        let db = self.store.db.clone();

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

        self.put_exec_state(state);

        let ret = self
            .modify_exec_state(|s| self.interpreter.begin(s))
            .await
            .expect("begin failed");

        to_begin_block(ret)
    }

    /// Apply a transaction to the application's state.
    async fn deliver_tx(&self, request: request::DeliverTx) -> response::DeliverTx {
        let msg = request.tx.to_vec();
        let result = self
            .modify_exec_state(|s| self.interpreter.deliver(s, msg))
            .await
            .expect("deliver failed");

        match result {
            Err(e) => invalid_deliver_tx(AppError::InvalidEncoding, e.description),
            Ok(ret) => match ret {
                ChainMessageApplyRet::Signed(Err(InvalidSignature(d))) => {
                    invalid_deliver_tx(AppError::InvalidSignature, d)
                }
                ChainMessageApplyRet::Signed(Ok(ret)) => to_deliver_tx(ret),
            },
        }
    }

    /// Signals the end of a block.
    async fn end_block(&self, _request: request::EndBlock) -> response::EndBlock {
        // TODO: Return events from epoch transitions.
        let ret = self
            .modify_exec_state(|s| self.interpreter.end(s))
            .await
            .expect("end failed");

        to_end_block(ret)
    }

    /// Commit the current state at the current height.
    async fn commit(&self) -> response::Commit {
        let exec_state = self.take_exec_state();
        let state_root = exec_state.commit().expect("failed to commit FVM");

        let mut state = self.store.committed_state();
        state.state_root = state_root;
        self.store.set_committed_state(state);

        // Reset check state.
        let mut guard = self.check_state.lock().await;
        *guard = None;

        response::Commit {
            data: state_root.to_bytes().into(),
            // We have to retain blocks until we can support Snapshots.
            retain_height: Default::default(),
        }
    }
}

/// Response to delivery where the input was blatantly invalid.
/// This indicates that the validator who made the block was Byzantine.
fn invalid_deliver_tx(err: AppError, description: String) -> response::DeliverTx {
    response::DeliverTx {
        code: Code::Err(NonZeroU32::try_from(err as u32).expect("error codes are non-zero")),
        info: description,
        ..Default::default()
    }
}

/// Response to check where the input was blatantly invalid.
/// This indicates that the user who sent the transaction is either attacking or has a faulty client.
fn invalid_check_tx(err: AppError, description: String) -> response::CheckTx {
    response::CheckTx {
        code: Code::Err(NonZeroU32::try_from(err as u32).expect("error codes are non-zero")),
        info: description,
        ..Default::default()
    }
}

fn to_deliver_tx(ret: FvmApplyRet) -> response::DeliverTx {
    let receipt = ret.apply_ret.msg_receipt;
    let code = to_code(receipt.exit_code);

    // Based on the sanity check in the `DefaultExecutor`.
    // gas_cost = gas_fee_cap * gas_limit; this is how much the account is charged up front.
    // &base_fee_burn + &over_estimation_burn + &refund + &miner_tip == gas_cost
    // But that's in tokens. I guess the closes to what we want is the limit.
    let gas_wanted: i64 = ret.gas_limit.try_into().expect("gas wanted not i64");
    let gas_used: i64 = receipt.gas_used.try_into().expect("gas used not i64");

    let data = receipt.return_data.to_vec().into();
    let events = to_events(ret.apply_ret.events);

    response::DeliverTx {
        code,
        data,
        log: Default::default(),
        info: Default::default(),
        gas_wanted,
        gas_used,
        events,
        codespace: Default::default(),
    }
}

fn to_check_tx(ret: FvmCheckRet) -> response::CheckTx {
    response::CheckTx {
        code: to_code(ret.exit_code),
        gas_wanted: ret.gas_limit.try_into().expect("gas wanted not i64"),
        sender: ret.sender.to_string(),
        ..Default::default()
    }
}

fn to_code(exit_code: ExitCode) -> Code {
    if exit_code.is_success() {
        Code::Ok
    } else {
        Code::Err(NonZeroU32::try_from(exit_code.value()).expect("error codes are non-zero"))
    }
}

/// Map the return values from epoch boundary operations to validator updates.
///
/// (Currently just a placeholder).
fn to_end_block(_ret: ()) -> response::EndBlock {
    response::EndBlock {
        validator_updates: Vec::new(),
        consensus_param_updates: None,
        events: Vec::new(),
    }
}

/// Map the return values from cron operations.
///
/// (Currently just a placeholder).
fn to_begin_block(ret: FvmApplyRet) -> response::BeginBlock {
    let events = to_events(ret.apply_ret.events);

    response::BeginBlock { events }
}

fn to_events(_stamped_events: Vec<StampedEvent>) -> Vec<Event> {
    // TODO: Convert events. This is currently not possible because the event fields are private.
    // I changed that in https://github.com/filecoin-project/ref-fvm/pull/1507 but it's still in review.
    // A possible workaround would be to retrieve the events by their CID, and use a custom type to parse.
    // It will be part of https://github.com/filecoin-project/ref-fvm/pull/1635 :)
    Vec::new()
}
