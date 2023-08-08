// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! IPC proof of finality related functions

use crate::parent::{ParentSyncer, PollingParentSyncer};
use crate::Config;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle;

type LockedProof = Arc<RwLock<IPCParentFinality>>;

/// The proof for POF.
#[derive(Clone, Serialize, Deserialize)]
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
pub struct ProofOfFinality {
    config: Config,
    started: Arc<AtomicBool>,
    latest_proof: LockedProof,

    handle: Option<JoinHandle<anyhow::Result<()>>>,
}

impl ProofOfFinality {
    pub fn get_proof(&self) -> IPCParentFinality {
        let finality = self.latest_proof.read().unwrap();
        finality.clone()
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

        let syncer = PollingParentSyncer {};
        let config = self.config.clone();
        let proof = self.latest_proof.clone();

        let handle = tokio::spawn(async move { sync_with_parent(config, syncer, proof).await });

        self.handle = Some(handle);

        Ok(())
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
    parent_syncer: T,
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
