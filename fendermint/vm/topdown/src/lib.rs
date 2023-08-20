// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod cache;
mod error;
mod finality;

use crate::error::Error;
use async_trait::async_trait;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use serde::{Deserialize, Serialize};

pub use crate::finality::DefaultFinalityProvider;

type BlockHeight = u64;
type Nonce = u64;
type Bytes = Vec<u8>;

#[derive(Debug, Clone)]
pub struct Config {
    /// The number of blocks to delay reporting when creating the pof
    chain_head_delay: BlockHeight,
    /// The lower bound for the chain head height in parent view
    chain_head_lower_bound: BlockHeight,

    /// Parent syncing cron period, in seconds
    polling_interval: u64,
}

/// The finality proof for IPC parent at certain height.
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

#[async_trait]
pub trait ParentViewProvider {
    /// Get the latest height of the parent recorded
    async fn latest_height(&self) -> Option<BlockHeight>;
    /// Get latest nonce recorded
    async fn latest_nonce(&self) -> Option<Nonce>;
    /// There is a new incoming parent view to be updated
    async fn new_parent_view(
        &self,
        block_info: Option<(BlockHeight, Bytes, ValidatorSet)>,
        top_down_msgs: Vec<CrossMsg>,
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait ParentFinalityProvider: ParentViewProvider {
    /// Obtains the last committed finality
    async fn last_committed_finality(&self) -> Result<IPCParentFinality, Error>;
    /// Latest proposal for parent finality
    async fn next_proposal(&self) -> Result<IPCParentFinality, Error>;
    /// Check if the target proposal is valid
    async fn check_proposal(&self, proposal: &IPCParentFinality) -> Result<(), Error>;
    /// Called when finality is committed
    async fn on_finality_committed(&self, finality: &IPCParentFinality) -> Result<(), Error>;
}
