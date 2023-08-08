// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Parent syncing related functions

use async_trait::async_trait;

/// Obtain the latest state information required for IPC from the parent subnet
#[async_trait]
pub trait ParentSyncer {
    /// Get the latest height
    async fn latest_height(&self) -> anyhow::Result<u64>;
    /// Get the block hash at the target height
    async fn block_hash(&self, height: u64) -> anyhow::Result<Vec<u8>>;
}

/// Constantly syncing with parent through polling
pub struct PollingParentSyncer {}

#[async_trait]
impl ParentSyncer for PollingParentSyncer {
    async fn latest_height(&self) -> anyhow::Result<u64> {
        todo!()
    }

    async fn block_hash(&self, _height: u64) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}
