// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod cache;
mod error;
mod finality;
pub mod sync;

pub mod convert;
pub mod proxy;
mod toggle;

use async_stm::Stm;
use async_trait::async_trait;
use ethers::utils::hex;
use fvm_shared::clock::ChainEpoch;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::staking::StakingChangeRequest;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::Duration;

pub use crate::cache::{SequentialAppendError, SequentialKeyCache, ValueIter};
pub use crate::error::Error;
pub use crate::finality::CachedFinalityProvider;
pub use crate::toggle::Toggle;

pub type BlockHeight = u64;
pub type Bytes = Vec<u8>;
pub type BlockHash = Bytes;

/// The null round error message
pub(crate) const NULL_ROUND_ERR_MSG: &str = "requested epoch was a null round";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// The number of blocks to delay before reporting a height as final on the parent chain.
    /// To propose a certain number of epochs delayed from the latest height, we see to be
    /// conservative and avoid other from rejecting the proposal because they don't see the
    /// height as final yet.
    pub chain_head_delay: BlockHeight,
    /// Extra delay on top of `chain_head_delay` before proposing a height as final on the parent chain,
    /// to avoid validator disagreeing by 1 height whether something is final or not just yet.
    pub proposal_delay: BlockHeight,
    /// Parent syncing cron period, in seconds
    pub polling_interval: Duration,
    /// Top down exponential back off retry base
    pub exponential_back_off: Duration,
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

impl IPCParentFinality {
    pub fn new(height: ChainEpoch, hash: BlockHash) -> Self {
        Self {
            height: height as BlockHeight,
            block_hash: hash,
        }
    }
}

impl Display for IPCParentFinality {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IPCParentFinality(height: {}, block_hash: {})",
            self.height,
            hex::encode(&self.block_hash)
        )
    }
}

#[async_trait]
pub trait ParentViewProvider {
    /// Obtain the genesis epoch of the current subnet in the parent
    fn genesis_epoch(&self) -> anyhow::Result<BlockHeight>;
    /// Get the validator changes from and to height.
    async fn validator_changes_from(
        &self,
        from: BlockHeight,
        to: BlockHeight,
    ) -> anyhow::Result<Vec<StakingChangeRequest>>;
    /// Get the validator changes at height.
    async fn validator_changes(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<Vec<StakingChangeRequest>>;
    /// Get the top down messages at height.
    async fn top_down_msgs(
        &self,
        height: BlockHeight,
        block_hash: &BlockHash,
    ) -> anyhow::Result<Vec<CrossMsg>>;
    /// Get the top down messages from and to height.
    async fn top_down_msgs_from(
        &self,
        from: BlockHeight,
        to: BlockHeight,
        block_hash: &BlockHash,
    ) -> anyhow::Result<Vec<CrossMsg>>;
}

pub trait ParentFinalityProvider: ParentViewProvider {
    /// Latest proposal for parent finality
    fn next_proposal(&self) -> Stm<Option<IPCParentFinality>>;
    /// Check if the target proposal is valid
    fn check_proposal(&self, proposal: &IPCParentFinality) -> Stm<bool>;
    /// Called when finality is committed
    fn set_new_finality(
        &self,
        finality: IPCParentFinality,
        previous_finality: Option<IPCParentFinality>,
    ) -> Stm<()>;
}

/// If res is null round error, returns the default value from f()
pub(crate) fn handle_null_round<T, F: FnOnce() -> T>(
    res: anyhow::Result<T>,
    f: F,
) -> anyhow::Result<T> {
    match res {
        Ok(t) => Ok(t),
        Err(e) => {
            if is_null_round_error(&e) {
                Ok(f())
            } else {
                Err(e)
            }
        }
    }
}

pub(crate) fn is_null_round_error(err: &anyhow::Error) -> bool {
    is_null_round_str(&err.to_string())
}

pub(crate) fn is_null_round_str(s: &str) -> bool {
    s.contains(NULL_ROUND_ERR_MSG)
}
