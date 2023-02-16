// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use std::future::Future;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cid::Cid;
use fendermint_abci::Application;
use fendermint_storage::{Codec, Encode, KVRead, KVReadable, KVStore, KVWritable, KVWrite};
use fendermint_vm_interpreter::bytes::{
    BytesMessageApplyRet, BytesMessageCheckRet, BytesMessageQuery, BytesMessageQueryRet,
};
use fendermint_vm_interpreter::chain::{ChainMessageApplyRet, IllegalMessage};
use fendermint_vm_interpreter::fvm::{
    FvmApplyRet, FvmCheckRet, FvmCheckState, FvmQueryRet, FvmQueryState, FvmState,
};
use fendermint_vm_interpreter::signed::InvalidSignature;
use fendermint_vm_interpreter::{CheckInterpreter, Interpreter, QueryInterpreter, Timestamp};
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::event::StampedEvent;
use fvm_shared::version::NetworkVersion;
use serde::{Deserialize, Serialize};
use tendermint::abci::request::CheckTxKind;
use tendermint::abci::{request, response, Code, Event, EventAttribute};

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

/// Handle ABCI requests.
#[derive(Clone)]
pub struct App<DB, S, I>
where
    DB: Blockstore + 'static,
    S: KVStore,
{
    db: Arc<DB>,
    /// Namespace to store app state.
    namespace: S::Namespace,
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
        Self {
            db: Arc::new(db),
            namespace,
            interpreter: Arc::new(interpreter),
            exec_state: Arc::new(Mutex::new(None)),
            check_state: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

impl<DB, S, I> App<DB, S, I>
where
    S: KVStore + Codec<AppState> + Encode<AppStoreKey>,
    DB: Blockstore + KVWritable<S> + KVReadable<S> + 'static + Clone,
{
    /// Get an owned clone of the database.
    fn clone_db(&self) -> DB {
        self.db.as_ref().clone()
    }
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
    I: QueryInterpreter<
        State = FvmQueryState<DB>,
        Query = BytesMessageQuery,
        Output = BytesMessageQueryRet,
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
    async fn query(&self, request: request::Query) -> response::Query {
        let db = self.clone_db();
        // TODO: Store the state for each height, or the last N heights, then use `request.height`.
        let state = self.committed_state();
        let block_height = state.block_height;
        let state = FvmQueryState::new(db, state.state_root).expect("error creating query state");
        let qry = (request.path, request.data.to_vec());

        let (_, result) = self
            .interpreter
            .query(state, qry)
            .await
            .expect("error running query");

        match result {
            Err(e) => invalid_query(AppError::InvalidEncoding, e.description),
            Ok(result) => to_query(result, block_height),
        }
    }

    /// Check the given transaction before putting it into the local mempool.
    async fn check_tx(&self, request: request::CheckTx) -> response::CheckTx {
        // Keep the guard through the check, so there can be only one at a time.
        let mut guard = self.check_state.lock().await;

        let state = guard.take().unwrap_or_else(|| {
            let db = self.clone_db();
            let state = self.committed_state();
            FvmCheckState::new(db, state.state_root).expect("error creating check state")
        });

        let (state, result) = self
            .interpreter
            .check(
                state,
                request.tx.to_vec(),
                request.kind == CheckTxKind::Recheck,
            )
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
        let db = self.clone_db();
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
        let block_height = exec_state.block_height();
        let state_root = exec_state.commit().expect("failed to commit FVM");

        let mut state = self.committed_state();
        state.state_root = state_root;
        state.block_height = block_height.try_into().expect("negative height");
        self.set_committed_state(state);

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

/// Response to checks where the input was blatantly invalid.
/// This indicates that the user who sent the transaction is either attacking or has a faulty client.
fn invalid_check_tx(err: AppError, description: String) -> response::CheckTx {
    response::CheckTx {
        code: Code::Err(NonZeroU32::try_from(err as u32).expect("error codes are non-zero")),
        info: description,
        ..Default::default()
    }
}

/// Response to queries where the input was blatantly invalid.
fn invalid_query(err: AppError, description: String) -> response::Query {
    response::Query {
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
    let gas_wanted: i64 = ret.gas_limit.try_into().unwrap_or(i64::MAX);
    let gas_used: i64 = receipt.gas_used.try_into().unwrap_or(i64::MAX);

    let data = receipt.return_data.to_vec().into();
    let events = to_events("message", ret.apply_ret.events);

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
        gas_wanted: ret.gas_limit.try_into().unwrap_or(i64::MAX),
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
    let events = to_events("begin", ret.apply_ret.events);

    response::BeginBlock { events }
}

/// Convert events to key-value pairs.
fn to_events(kind: &str, stamped_events: Vec<StampedEvent>) -> Vec<Event> {
    stamped_events
        .into_iter()
        .map(|se| {
            let mut attrs = Vec::new();

            attrs.push(EventAttribute {
                key: "emitter".to_string(),
                value: se.emitter.to_string(),
                index: true,
            });

            for e in se.event.entries {
                attrs.push(EventAttribute {
                    key: e.key,
                    value: hex::encode(e.value),
                    index: !e.flags.is_empty(),
                });
            }

            Event::new(kind.to_string(), attrs)
        })
        .collect()
}

/// Map to query results.
fn to_query(ret: FvmQueryRet, block_height: u64) -> response::Query {
    let exit_code = match ret {
        FvmQueryRet::Ipld(None) | FvmQueryRet::ActorState(None) => ExitCode::USR_NOT_FOUND,
        FvmQueryRet::Ipld(_) | FvmQueryRet::ActorState(_) => ExitCode::OK,
    };

    // The return value has a `key` field which is supposed to be set to the data matched.
    // Although at this point I don't have access to the input like the CID looked up,
    // but I assume the query sender has. Rather than repeat everything, I'll add the key
    // where it gives some extra information, like the actor ID, just to keep this option visible.
    let key = match &ret {
        FvmQueryRet::ActorState(Some((id, _))) => fvm_ipld_encoding::to_vec(id).unwrap(),
        _ => Vec::new(),
    };

    let value = match ret {
        FvmQueryRet::Ipld(None) | FvmQueryRet::ActorState(None) => Vec::new(),
        FvmQueryRet::Ipld(Some(bz)) => bz,
        FvmQueryRet::ActorState(Some((_, st))) => fvm_ipld_encoding::to_vec(&st).unwrap(),
    };

    // The height here is the height of the block that was committed, not in which the app hash appeared,
    // so according to Tendermint docstrings we need to return plus one.
    let height = tendermint::block::Height::try_from(block_height + 1).expect("height too big");

    response::Query {
        code: to_code(exit_code),
        key: key.into(),
        value: value.into(),
        height,
        ..Default::default()
    }
}
