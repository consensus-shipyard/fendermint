// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::time::SystemTime;

use async_stm::Stm;
use fendermint_vm_interpreter::fvm::state::{
    snapshot::{BlockHeight, SnapshotVersion},
    FvmStateParams,
};

use crate::{state::SnapshotState, SnapshotItem};

/// Interface to snapshot state for the application.
#[derive(Clone)]
pub struct SnapshotClient {
    /// The client will only notify the manager of snapshottable heights.
    snapshot_interval: BlockHeight,
    state: SnapshotState,
}

impl SnapshotClient {
    pub fn new(snapshot_interval: BlockHeight, state: SnapshotState) -> Self {
        Self {
            snapshot_interval,
            state,
        }
    }
    /// Set the latest block state parameters and notify the manager.
    ///
    /// Call this with the block height where the `app_hash` in the block reflects the
    /// state in the parameters, that is, the in the *next* block.
    pub fn notify(&self, block_height: BlockHeight, state_params: FvmStateParams) -> Stm<()> {
        if block_height % self.snapshot_interval == 0 {
            self.state
                .latest_params
                .write(Some((state_params, block_height)))?;
        }
        Ok(())
    }

    /// List completed snapshots.
    pub fn list_snapshots(&self) -> Stm<im::Vector<SnapshotItem>> {
        self.state.snapshots.read_clone()
    }

    /// Try to find a snapshot, if it still exists.
    ///
    /// If found, mark it as accessed, so that it doesn't get purged while likely to be requested or read from disk.
    pub fn access_snapshot(
        &self,
        block_height: BlockHeight,
        version: SnapshotVersion,
    ) -> Stm<Option<SnapshotItem>> {
        let mut snapshots = self.state.snapshots.read_clone()?;
        let mut snapshot = None;
        for s in snapshots.iter_mut() {
            if s.manifest.block_height == block_height && s.manifest.version == version {
                s.last_access = SystemTime::now();
                snapshot = Some(s.clone());
                break;
            }
        }
        if snapshot.is_some() {
            self.state.snapshots.write(snapshots)?;
        }
        Ok(snapshot)
    }
}
