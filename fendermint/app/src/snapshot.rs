// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::BlockHeight;
use fendermint_vm_interpreter::fvm::state::{FvmStateParams, Snapshot};
use fvm_ipld_blockstore::Blockstore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The config params for snapshot
#[derive(Clone)]
pub struct SnapshotConfig {
    path: String,
    period: BlockHeight,
}

/// Manager the snapshot of the entire state tree and state params at certain blocks.
#[derive(Clone)]
pub struct SnapshotManager {
    config: SnapshotConfig,
    lock: Arc<AtomicBool>,
}

const STARTED: bool = true;

impl SnapshotManager {
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            config,
            lock: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn should_run(&self, height: BlockHeight) -> bool {
        height % self.config.period == 0
    }

    pub fn start<BS: Blockstore + 'static + Send>(
        &self,
        block_height: BlockHeight,
        state_params: FvmStateParams,
        store: BS,
    ) -> bool {
        tracing::debug!("attempting to trigger snapshot at height: {block_height}");

        let snapshot = match Snapshot::new(store, state_params, block_height) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("cannot start snapshot at block: {block_height} due to: {e}");
                return false;
            }
        };

        match self
            .lock
            .compare_exchange(!STARTED, STARTED, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {}
            Err(_) => {
                tracing::info!(
                    "triggering snapshot at height: {block_height} fails as it's already running"
                );
                return false;
            }
        };

        let path = self.config.path.clone();
        let lock = self.lock.clone();
        tokio::spawn(async move {
            let r = snapshot.write_car(path).await;
            lock.store(!STARTED, Ordering::SeqCst);
            r
        });

        true
    }
}
