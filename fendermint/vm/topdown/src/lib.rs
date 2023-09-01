// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod cache;
mod error;
mod finality;
mod sync;

use async_stm::StmDynResult;
use ipc_agent_sdk::message::ipc::ValidatorSet;
use ipc_sdk::cross::CrossMsg;
use serde::{Deserialize, Serialize};

pub use crate::cache::{SequentialAppendError, SequentialKeyCache, ValueIter};
pub use crate::error::Error;
pub use crate::finality::InMemoryFinalityProvider;
pub use crate::sync::{IPCAgentProxy, PollingParentSyncer};

type BlockHeight = u64;
type Bytes = Vec<u8>;
type BlockHash = Bytes;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// The number of blocks to delay before reporting a height as final on the parent chain.
    /// To propose a certain number of epochs delayed from the latest height, we see to be
    /// conservative and avoid other from rejecting the proposal because they don't see the
    /// height as final yet.
    pub chain_head_delay: BlockHeight,
    /// Parent syncing cron period, in seconds
    pub polling_interval_secs: u64,
    /// The ipc agent url
    pub ipc_agent_url: String,
}

/// The finality view for IPC parent at certain height.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IPCParentFinality {
    /// The latest chain height
    pub height: BlockHeight,
    /// The block hash. For FVM, it is a Cid. For Evm, it is bytes32 as one can now potentially
    /// deploy a subnet on EVM.
    pub block_hash: BlockHash,
}

pub trait ParentViewProvider {
    /// Get the latest height of the parent recorded
    fn latest_height(&self) -> StmDynResult<Option<BlockHeight>>;
    /// Get the block hash at height
    fn block_hash(&self, height: BlockHeight) -> StmDynResult<Option<BlockHash>>;
    /// Get the validator set at height
    fn validator_set(&self, height: BlockHeight) -> StmDynResult<Option<ValidatorSet>>;
    /// Get the top down messages at height
    fn top_down_msgs(&self, height: BlockHeight) -> StmDynResult<Vec<CrossMsg>>;
    /// There is a new parent view is ready to be updated
    fn new_parent_view(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_set: ValidatorSet,
        top_down_msgs: Vec<CrossMsg>,
    ) -> StmDynResult<()>;
}

pub trait ParentFinalityProvider: ParentViewProvider {
    /// Obtains the last committed finality
    fn last_committed_finality(&self) -> StmDynResult<IPCParentFinality>;
    /// Latest proposal for parent finality
    fn next_proposal(&self) -> StmDynResult<Option<IPCParentFinality>>;
    /// Check if the target proposal is valid
    fn check_proposal(&self, proposal: &IPCParentFinality) -> StmDynResult<()>;
    /// Called when finality is committed
    fn on_finality_committed(&self, finality: &IPCParentFinality) -> StmDynResult<()>;
}
