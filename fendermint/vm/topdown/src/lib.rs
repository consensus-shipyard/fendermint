// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod cache;
mod error;
mod finality;
pub mod sync;

#[cfg(feature = "conversion")]
pub mod convert;
mod toggle;

use async_stm::{Stm, StmResult};
use async_trait::async_trait;
use ipc_agent_sdk::message::ipc::ValidatorSet;
use ipc_sdk::cross::CrossMsg;
use serde::{Deserialize, Serialize};

pub use crate::cache::{SequentialAppendError, SequentialKeyCache, ValueIter};
pub use crate::error::Error;
pub use crate::finality::CachedFinalityProvider;
pub use crate::toggle::Toggle;

pub type BlockHeight = u64;
pub type Bytes = Vec<u8>;
pub type BlockHash = Bytes;

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
    /// Max block parent view block gap allowed
    pub max_parent_view_block_gap: BlockHeight,
    /// Top down exponential back off retry base
    pub exponential_back_off_secs: u64,
    /// The max number of retries for exponential backoff before giving up
    pub exponential_retry_limit: usize,
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

#[async_trait]
pub trait ParentViewProvider {
    /// Get the validator set at height
    async fn validator_set(&self, height: BlockHeight) -> StmResult<Option<ValidatorSet>, Error>;
    /// Get the top down messages at height
    async fn top_down_msgs(&self, height: BlockHeight) -> StmResult<Option<Vec<CrossMsg>>, Error>;
}

pub trait ParentFinalityProvider: ParentViewProvider {
    /// Latest proposal for parent finality
    fn next_proposal(&self) -> Stm<Option<IPCParentFinality>>;
    /// Check if the target proposal is valid
    fn check_proposal(&self, proposal: &IPCParentFinality) -> Stm<bool>;
    /// Called when finality is committed
    fn set_new_finality(&self, finality: IPCParentFinality) -> Stm<()>;
}
