// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod cache;
mod error;
mod finality;

use async_stm::StmDynResult;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use serde::{Deserialize, Serialize};

pub use crate::cache::{SequentialAppendError, SequentialKeyCache, ValueIter};
pub use crate::finality::DefaultFinalityProvider;

type BlockHeight = u64;
type Nonce = u64;
type Bytes = Vec<u8>;
type BlockHash = Bytes;

#[derive(Debug, Clone)]
pub struct Config {
    /// The number of blocks to delay before reporting a height as final on the parent chain.
    /// To propose a certain number of epochs delayed from the latest height, we see to be
    /// conservative and avoid other from rejecting the proposal because they don't see the
    /// height as final yet.
    chain_head_delay: BlockHeight,
    /// The top-down block proposal height interval. Anything in-between these heights is ignored.
    block_interval: BlockHeight,
    /// Parent syncing cron period, in seconds
    polling_interval_secs: u64,
}

/// The finality view for IPC parent at certain height.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IPCParentFinality {
    /// The latest chain height
    pub height: BlockHeight,
    /// The block hash. For FVM, it is a Cid. For Evm, it is bytes32 as one can now potentially
    /// deploy a subnet on EVM.
    pub block_hash: Vec<u8>,
    /// new top-down messages finalized in this PoF
    pub top_down_msgs: Vec<CrossMsg>,
    /// latest validator configuration information from the parent.
    pub validator_set: ValidatorSet,
}

pub trait ParentViewProvider {
    /// Get the latest height of the parent recorded
    fn latest_height(&self) -> StmDynResult<Option<BlockHeight>>;
    /// Get latest nonce recorded
    fn latest_nonce(&self) -> StmDynResult<Option<Nonce>>;
    /// There is a new parent view is ready to be updated
    fn new_parent_view(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_set: ValidatorSet,
        top_down_msgs: Vec<CrossMsg>
    ) -> StmDynResult<()>;
}

pub trait ParentFinalityProvider: ParentViewProvider {
    /// Obtains the last committed finality
    fn last_committed_finality(&self) -> StmDynResult<IPCParentFinality>;
    /// Latest proposal for parent finality
    fn next_proposal(&self) -> StmDynResult<IPCParentFinality>;
    /// Check if the target proposal is valid
    fn check_proposal(&self, proposal: &IPCParentFinality) -> StmDynResult<()>;
    /// Called when finality is committed
    fn on_finality_committed(&self, finality: &IPCParentFinality) -> StmDynResult<()>;
}
