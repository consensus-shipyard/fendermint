// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! Interfacing with IPC, provides utility functions

mod agent;
mod cache;
mod message;
pub mod parent;
pub mod pof;

use crate::pof::IPCParentFinality;
use async_trait::async_trait;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
pub use message::IPCMessage;

#[derive(Debug, Clone)]
pub struct Config {
    /// The number of blocks to delay reporting when creating the pof
    chain_head_delay: BlockHeight,
    /// The lower bound for the chain head height in parent view
    chain_head_lower_bound: BlockHeight,

    /// Parent syncing cron period, in seconds
    polling_interval: u64,
}

type BlockHeight = u64;
type Nonce = u64;

/// Obtain the latest state information required for IPC from the parent subnet
pub trait ParentViewProvider {
    /// Fetch the latest chain head
    fn latest_height(&self) -> Option<BlockHeight>;
    /// Fetch the block hash at target height
    fn block_hash(&self, height: BlockHeight) -> Option<Vec<u8>>;
    /// Get the top down messages from the nonce of a height
    fn top_down_msgs(&self, height: BlockHeight, nonce: u64) -> Vec<CrossMsg>;
    /// Get the latest membership information
    fn membership(&self) -> Option<ValidatorSet>;

    /// Called when finality is committed
    fn on_finality_committed(&self, finality: &IPCParentFinality);
}

/// Obtain the latest state information required for IPC from the parent subnet
#[async_trait]
pub trait ParentViewQuery {
    /// Fetch the latest chain head
    async fn latest_height(&self) -> anyhow::Result<BlockHeight>;
    /// Fetch the block hash at target height
    async fn block_hash(&self, height: BlockHeight) -> anyhow::Result<Option<Vec<u8>>>;
    /// Get the top down messages from the nonce of a height
    async fn top_down_msgs(&self, height: BlockHeight, nonce: u64)
        -> anyhow::Result<Vec<CrossMsg>>;
    /// Get the latest membership information
    async fn membership(&self) -> anyhow::Result<ValidatorSet>;
}
