// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use async_trait::async_trait;
use cid::Cid;
use fvm::state_tree::ActorState;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::serde::{Deserialize, Serialize};
use fvm_shared::{address::Address, ActorID};

use crate::QueryInterpreter;

use super::{state::FvmQueryState, FvmMessageInterpreter};

/// Queries over the IPLD blockstore or the state tree.
///
/// Maybe we can have some common queries over the known state of built-in actors,
/// and actors supporting IPC, or FEVM.
#[derive(Serialize, Deserialize)]
pub enum FvmQuery {
    /// Query something from the IPLD store.
    Ipld(Cid),
    /// Query the state of an actor.
    ActorState(Address),
}

pub enum FvmQueryRet {
    /// Bytes from the IPLD store retult, if found.
    Ipld(Option<Vec<u8>>),
    /// The full state of an actor, if found.
    ActorState(Option<(ActorID, ActorState)>),
}

#[async_trait]
impl<DB> QueryInterpreter for FvmMessageInterpreter<DB>
where
    DB: Blockstore + 'static + Send + Sync + Clone,
{
    type State = FvmQueryState<DB>;
    type Query = FvmQuery;
    type Output = FvmQueryRet;

    async fn query(
        &self,
        state: Self::State,
        qry: Self::Query,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        let res = match qry {
            FvmQuery::Ipld(cid) => FvmQueryRet::Ipld(state.store_get(&cid)?),
            FvmQuery::ActorState(addr) => FvmQueryRet::ActorState(state.actor_state(&addr)?),
        };
        Ok((state, res))
    }
}
