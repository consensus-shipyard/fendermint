// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{
    BlockHash, BlockHeight, Error, IPCParentFinality, ParentFinalityProvider, ParentViewProvider,
};
use async_stm::{StmError, StmResult};
use ipc_agent_sdk::message::ipc::ValidatorSet;
use ipc_sdk::cross::CrossMsg;

/// The parent finality provider could have all functionalities disabled.
#[derive(Clone)]
pub struct Toggle<P> {
    inner: Option<P>,
}

impl<P> Toggle<P> {
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    pub fn enabled(inner: P) -> Self {
        Self { inner: Some(inner) }
    }

    fn perform<F, T>(&self, f: F) -> StmResult<T, Error>
    where
        F: FnOnce(&P) -> StmResult<T, Error>,
    {
        match &self.inner {
            Some(p) => f(p),
            None => Err(StmError::Abort(Error::ProviderNotEnabled)),
        }
    }
}

impl<P: ParentViewProvider> ParentViewProvider for Toggle<P> {
    fn latest_height(&self) -> StmResult<Option<BlockHeight>, Error> {
        self.perform(|p| p.latest_height())
    }

    fn block_hash(&self, height: BlockHeight) -> StmResult<Option<BlockHash>, Error> {
        self.perform(|p| p.block_hash(height))
    }

    fn validator_set(&self, height: BlockHeight) -> StmResult<Option<ValidatorSet>, Error> {
        self.perform(|p| p.validator_set(height))
    }

    fn top_down_msgs(&self, height: BlockHeight) -> StmResult<Vec<CrossMsg>, Error> {
        self.perform(|p| p.top_down_msgs(height))
    }

    fn new_parent_view(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_set: ValidatorSet,
        top_down_msgs: Vec<CrossMsg>,
    ) -> StmResult<(), Error> {
        self.perform(|p| p.new_parent_view(height, block_hash, validator_set, top_down_msgs))
    }
}

impl<P: ParentFinalityProvider> ParentFinalityProvider for Toggle<P> {
    fn last_committed_finality(&self) -> StmResult<IPCParentFinality, Error> {
        self.perform(|p| p.last_committed_finality())
    }

    fn next_proposal(&self) -> StmResult<Option<IPCParentFinality>, Error> {
        self.perform(|p| p.next_proposal())
    }

    fn check_proposal(&self, proposal: &IPCParentFinality) -> StmResult<(), Error> {
        self.perform(|p| p.check_proposal(proposal))
    }

    fn set_new_finality(&self, finality: IPCParentFinality) -> StmResult<(), Error> {
        self.perform(|p| p.set_new_finality(finality))
    }
}
