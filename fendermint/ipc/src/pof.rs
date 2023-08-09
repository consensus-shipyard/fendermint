// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! IPC proof of finality related functions

use crate::parent::ParentSyncer;
use crate::Config;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle;

type LockedProof = Arc<RwLock<IPCParentFinality>>;

/// The proof for POF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCParentFinality {
    /// The latest chain height
    height: u64,
    /// The block hash. For FVM, it is a Cid. For Evm, it is bytes32.
    block_hash: Vec<u8>,
    // /// new top-down messages finalized in this PoF
    // top_down_msgs: Vec<CrossMsg>,
    // /// latest configuration information from the parent.
    // config: MembershipSet,
}

/// The proof of finality util struct
pub struct ProofOfFinality<T> {
    config: Config,
    started: Arc<AtomicBool>,
    latest_proof: LockedProof,
    parent_syncer: Arc<T>,

    handle: Option<JoinHandle<anyhow::Result<()>>>,
}

impl<T: ParentSyncer + Send + Sync + 'static> ProofOfFinality<T> {
    pub fn get_finality(&self) -> IPCParentFinality {
        let finality = self.latest_proof.read().unwrap();
        finality.clone()
    }

    pub async fn check_finality(&self, other_finality: &IPCParentFinality) -> bool {
        if !self.check_height(other_finality.height) {
            return false;
        }

        // Check the block hash. If we cannot reach the parent to get the target height, we cannot
        // verify the finality.
        let hash_match = match self.parent_syncer.block_hash(other_finality.height).await {
            Ok(hash) => hash == other_finality.block_hash,
            Err(e) => {
                tracing::warn!(
                    "cannot get block hash at height: {} due to: {e}",
                    other_finality.height
                );
                false
            }
        };

        if !hash_match {
            return false;
        }

        // TODO: add checks for top down messages and membership set

        true
    }

    /// Start the proof of finality listener in the background
    pub fn start(&mut self) -> anyhow::Result<()> {
        match self
            .started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {}
            Err(_) => return Err(anyhow!("already started")),
        }

        let parent_syncer = self.parent_syncer.clone();
        let config = self.config.clone();
        let proof = self.latest_proof.clone();

        let handle =
            tokio::spawn(async move { sync_with_parent(config, parent_syncer, proof).await });

        self.handle = Some(handle);

        Ok(())
    }

    fn check_height(&self, other_height: u64) -> bool {
        let finality = self.latest_proof.read().unwrap();

        // the `height` in finality is slower than the heaviest height by `chain_head_delay`
        // add it back to get the latest chain head height.
        let heaviest = finality.height + self.config.chain_head_delay;
        if heaviest < other_height {
            tracing::debug!(
                "other finality height: {:?} is ahead of own parent view heaviest: {:?}",
                other_height,
                heaviest
            );
            return false;
        }
        true
    }
}

macro_rules! log_error_only {
    ($r:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("cannot sync with parent: {e}");
                continue;
            }
        }
    };
}

async fn sync_with_parent<T: ParentSyncer>(
    config: Config,
    parent_syncer: Arc<T>,
    lock: LockedProof,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(config.polling_interval));

    loop {
        interval.tick().await;

        let latest_height = log_error_only!(parent_syncer.latest_height().await);
        if latest_height < config.chain_head_delay {
            continue;
        }

        let height_to_fetch = latest_height - config.chain_head_delay;
        let block_hash = parent_syncer.block_hash(height_to_fetch).await?;

        let candidate = IPCParentFinality {
            height: height_to_fetch,
            block_hash,
        };

        let mut proof = lock.write().unwrap();
        *proof = candidate;
    }
}
