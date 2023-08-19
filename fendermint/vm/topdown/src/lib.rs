// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod cache;
mod sync;

use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use serde::{Deserialize, Serialize};

type BlockHeight = u64;

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

/// Obtain the latest state information required for IPC from the parent subnet
pub trait ParentViewProvider {
    /// Fetch the latest chain head
    fn latest_height(&self) -> Option<BlockHeight>;
    /// Fetch the block hash at target height
    fn block_hash(&self, height: BlockHeight) -> Option<Vec<u8>>;
    /// Get the top down messages from the nonce of a height
    fn top_down_msgs(&self, height: BlockHeight, nonce: u64) -> Vec<CrossMsg>;
    /// Get the latest membership information
    fn validator_set(&self) -> Option<ValidatorSet>;

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
