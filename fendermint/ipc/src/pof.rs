// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! IPC proof of finality related functions

use crate::{BlockHeight, ParentViewProvider};
use crate::Config;
use anyhow::anyhow;
use cid::multihash::{Code, MultihashDigest};
use cid::Cid;
use fvm_ipld_encoding::DAG_CBOR;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle;

/// The proof for POF.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IPCParentFinality {
    /// The latest chain height
    pub height: u64,
    /// The block hash. For FVM, it is a Cid. For Evm, it is bytes32.
    pub block_hash: Vec<u8>,
    /// new top-down messages finalized in this PoF
    pub top_down_msgs: Vec<CrossMsg>,
    /// latest configuration information from the parent.
    pub config: ValidatorSet,
}

/// The parent finality information provider. It provides the latest finality information about the
/// parent. Also it perform validation on incoming finality.
#[derive(Clone)]
pub struct ParentFinalityProvider<T> {
    config: Config,
    parent_view_provider: Arc<T>,
    latest_confirmed_finality: Option<(Cid, IPCParentFinality)>,
}

impl<T: ParentViewProvider> ParentFinalityProvider<T> {
    pub fn finality_committed(&mut self, finality: IPCParentFinality) -> anyhow::Result<()> {
        let cid = derive_cid(&finality)?;
        self.parent_view_provider.on_finality_committed(&finality);
        self.latest_confirmed_finality.replace((cid, finality));
        Ok(())
    }

    /// The next finality to propose
    pub fn next_finality_proposal(&self) -> anyhow::Result<Option<IPCParentFinality>> {
        let latest_height = match self.parent_view_provider.latest_height() {
            Some(h) => h,
            None => return Ok(None),
        };
        if latest_height < self.config.chain_head_delay {
            return Ok(None);
        }

        let confident_height = latest_height - self.config.chain_head_delay;
        self.finality_proposal_at_height(confident_height)
    }

    pub fn finality_proposal_at_height(&self, height: BlockHeight) -> anyhow::Result<Option<IPCParentFinality>> {
        let hash = match self.parent_view_provider.block_hash(height) {
            None => return Err(anyhow!("block hash cannot be fetched at {height}")),
            Some(hash) => hash,
        };

        // TODO: handle top down messages and membership set
        Ok(Some(IPCParentFinality {
            height: confident_height,
            block_hash: hash,
            top_down_msgs: vec![],
            config: Default::default(),
        }))
    }

    pub fn check_finality(&self, other_finality: &IPCParentFinality) -> bool {
        // TODO: create the finality at target height
        let this_finality = match self.next_finality_proposal() {
            Ok(Some(finality)) => finality,
            _ => {
                tracing::info!("cannot create next finality, check return false");
                return false;
            }
        };

        let this_cid = match derive_cid(&this_finality) {
            Ok(cid) => cid,
            Err(e) => {
                tracing::error!("cannot derive cid for self created finality, report bug. Error {e}, finality: {this_finality:?}");
                return false;
            }
        };
        let other_cid = match derive_cid(other_finality) {
            Ok(cid) => cid,
            Err(_) => {
                tracing::info!("cannot derive cid from other finality, check return false");
                return false;
            }
        };

        this_cid == other_cid
    }

    /// Checks if the incoming parent finality is valid
    fn check_finality_v2(&self, other_finality: &IPCParentFinality) -> bool {
        if !self.check_height(other_finality.height) {
            return false;
        }

        if !self.check_hash(other_finality) {
            return false;
        }

        if !self.check_top_down_msgs(other_finality) {
            return false;
        }

        self.check_membership(other_finality)
    }
}

impl<T: ParentViewProvider> ParentFinalityProvider<T> {
    fn check_height(&self, other_height: u64) -> bool {
        let heaviest = self.parent_view_provider.latest_height();
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

    fn check_hash(&self, other_finality: &IPCParentFinality) -> bool {
        match self.parent_view_provider.block_hash(other_finality.height) {
            Some(hash) => hash == other_finality.block_hash,
            None => {
                // If we cannot reach the parent to get the target height, we cannot
                // verify the finality.
                tracing::info!("cannot get block hash at height: {}", other_finality.height);
                false
            }
        }
    }

    fn check_top_down_msgs(&self, _other_finality: &IPCParentFinality) -> bool {
        todo!()
    }

    fn check_membership(&self, _other_finality: &IPCParentFinality) -> bool {
        todo!()
    }
}

fn derive_cid<T: Serialize>(t: &T) -> anyhow::Result<Cid> {
    let bytes = fvm_ipld_encoding::to_vec(t)?;
    Ok(Cid::new_v1(DAG_CBOR, Code::Blake2b256.digest(&bytes)))
}
