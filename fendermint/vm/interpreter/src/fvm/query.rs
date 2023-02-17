// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use async_trait::async_trait;
use fendermint_vm_message::query::{FvmQuery, FvmQueryRet};
use fvm_ipld_blockstore::Blockstore;

use crate::QueryInterpreter;

use super::{state::FvmQueryState, FvmMessageInterpreter};

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
            FvmQuery::ActorState(addr) => {
                FvmQueryRet::ActorState(state.actor_state(&addr)?.map(Box::new))
            }
        };
        Ok((state, res))
    }
}
