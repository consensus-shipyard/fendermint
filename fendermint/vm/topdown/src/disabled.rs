// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{
    BlockHash, BlockHeight, Error, IPCParentFinality, InMemoryFinalityProvider,
    ParentFinalityProvider, ParentViewProvider,
};
use async_stm::{StmError, StmResult};
use ipc_agent_sdk::message::ipc::ValidatorSet;
use ipc_sdk::cross::CrossMsg;

/// The parent finality provider could have all functionalities disabled.
#[derive(Clone)]
pub enum MaybeDisabledProvider {
    Disabled,
    Enabled(InMemoryFinalityProvider),
}

impl MaybeDisabledProvider {
    pub fn disabled() -> Self {
        Self::Disabled
    }

    pub fn enabled(inner: InMemoryFinalityProvider) -> Self {
        Self::Enabled(inner)
    }
}

macro_rules! disabled {
    () => {
        Err(StmError::Abort(Error::ProviderNotEnabled))
    };
}

impl ParentViewProvider for MaybeDisabledProvider {
    fn latest_height(&self) -> StmResult<Option<BlockHeight>, Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.latest_height(),
        }
    }

    fn block_hash(&self, height: BlockHeight) -> StmResult<Option<BlockHash>, Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.block_hash(height),
        }
    }

    fn validator_set(&self, height: BlockHeight) -> StmResult<Option<ValidatorSet>, Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.validator_set(height),
        }
    }

    fn top_down_msgs(&self, height: BlockHeight) -> StmResult<Vec<CrossMsg>, Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.top_down_msgs(height),
        }
    }

    fn new_parent_view(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_set: ValidatorSet,
        top_down_msgs: Vec<CrossMsg>,
    ) -> StmResult<(), Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => {
                p.new_parent_view(height, block_hash, validator_set, top_down_msgs)
            }
        }
    }
}

impl ParentFinalityProvider for MaybeDisabledProvider {
    fn last_committed_finality(&self) -> StmResult<IPCParentFinality, Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.last_committed_finality(),
        }
    }

    fn next_proposal(&self) -> StmResult<Option<IPCParentFinality>, Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.next_proposal(),
        }
    }

    fn check_proposal(&self, proposal: &IPCParentFinality) -> StmResult<(), Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.check_proposal(proposal),
        }
    }

    fn set_new_finality(&self, finality: IPCParentFinality) -> StmResult<(), Error> {
        match self {
            MaybeDisabledProvider::Disabled => disabled!(),
            MaybeDisabledProvider::Enabled(p) => p.set_new_finality(finality),
        }
    }
}
