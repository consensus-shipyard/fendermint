// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::PathBuf;
use std::time::Duration;

use async_stm::{atomically, retry, TVar};
use fendermint_vm_interpreter::fvm::state::snapshot::{BlockHeight, BlockStateParams};
use fendermint_vm_interpreter::fvm::state::FvmStateParams;
use tendermint_rpc::Client;

const SYNC_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// State of snapshots, including the list of available completed ones
/// and the next eligible height.
#[derive(Clone)]
struct SnapshotState {
    /// Snapshotted heights are a multiple of the interval.
    ///
    /// The manager is free to skip heights if it's busy.
    snapshot_interval: BlockHeight,
    /// Location to store completed snapshots.
    _snapshot_dir: PathBuf,
    /// The latest state parameters at a snapshottable height.
    latest_params: TVar<Option<BlockStateParams>>,
}

/// Interface to snapshot state for the application.
#[derive(Clone)]
pub struct SnapshotClient {
    snapshot_state: SnapshotState,
}

impl SnapshotClient {
    /// Set the latest block state parameters and notify the manager.
    pub async fn on_commit(&self, block_height: BlockHeight, params: FvmStateParams) {
        if block_height % self.snapshot_state.snapshot_interval == 0 {
            atomically(|| {
                self.snapshot_state
                    .latest_params
                    .write(Some((params.clone(), block_height)))
            })
            .await;
        }
    }
}

/// Create snapshots at regular block intervals.
pub struct SnapshotManager {
    /// Shared state of snapshots.
    snapshot_state: SnapshotState,
    /// Indicate whether CometBFT has finished syncing with the chain,
    /// so that we can skip snapshotting old states while catching up.
    is_syncing: TVar<bool>,
}

impl SnapshotManager {
    /// Create a new manager.
    pub fn new(snapshot_interval: BlockHeight, snapshot_dir: PathBuf) -> Self {
        Self {
            snapshot_state: SnapshotState {
                snapshot_interval,
                _snapshot_dir: snapshot_dir,
                // Start with nothing to snapshot until we are notified about a new height.
                // We could also look back to find the latest height we should have snapshotted.
                latest_params: TVar::new(None),
            },
            // Assume we are syncing until we can determine otherwise.
            is_syncing: TVar::new(true),
        }
    }

    /// Create a client to talk to this manager.
    pub fn snapshot_client(&self) -> SnapshotClient {
        SnapshotClient {
            snapshot_state: self.snapshot_state.clone(),
        }
    }

    /// Produce snapshots.
    pub async fn run<C>(self, client: C)
    where
        C: Client + Send + Sync + 'static,
    {
        // Start a background poll to CometBFT.
        // We could just do this once and await here, but this way ostensibly CometBFT could be
        // restarted without Fendermint and go through another catch up.
        {
            let is_syncing = self.is_syncing.clone();
            tokio::spawn(async {
                poll_sync_status(client, is_syncing).await;
            });
        }

        let mut last_params = None;
        loop {
            let new_params = atomically(|| {
                // Check the current sync status. We could just query the API, but then we wouldn't
                // be notified when we finally reach the end, and we'd only snapshot the next height,
                // not the last one as soon as the chain is caught up.
                if *self.is_syncing.read()? {
                    retry()?;
                }

                match self.snapshot_state.latest_params.read()?.as_ref() {
                    None => retry()?,
                    unchanged if *unchanged == last_params => retry()?,
                    Some(new_params) => Ok(new_params.clone()),
                }
            })
            .await;

            last_params = Some(new_params.clone());

            self.create_snapshot(new_params).await;
        }
    }

    async fn create_snapshot(&self, (_params, _height): BlockStateParams) {
        todo!()
    }
}

/// Periodically ask CometBFT if it has caught up with the chain.
async fn poll_sync_status<C>(client: C, is_syncing: TVar<bool>)
where
    C: Client + Send + Sync + 'static,
{
    loop {
        match client.status().await {
            Ok(status) => {
                let catching_up = status.sync_info.catching_up;

                atomically(|| {
                    if *is_syncing.read()? != catching_up {
                        is_syncing.write(catching_up)?;
                    }
                    Ok(())
                })
                .await;
            }
            Err(e) => {
                tracing::warn!(error =? e, "failed to poll CometBFT sync status");
            }
        }
        tokio::time::sleep(SYNC_POLL_INTERVAL).await;
    }
}
