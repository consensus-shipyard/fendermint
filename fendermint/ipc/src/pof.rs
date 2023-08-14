// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! IPC proof of finality related functions

use crate::Config;
use crate::{BlockHeight, ParentViewProvider};
use anyhow::anyhow;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    latest_confirmed_finality: Option<IPCParentFinality>,
}

impl<T: ParentViewProvider> ParentFinalityProvider<T> {
    pub fn finality_committed(&mut self, finality: IPCParentFinality) -> anyhow::Result<()> {
        self.parent_view_provider.on_finality_committed(&finality);
        self.latest_confirmed_finality.replace(finality);
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

    pub fn finality_proposal_at_height(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<Option<IPCParentFinality>> {
        let block_hash = match self.parent_view_provider.block_hash(height) {
            None => return Err(anyhow!("block hash cannot be fetched at {height}")),
            Some(hash) => hash,
        };

        // TODO: handle top down messages and membership set
        Ok(Some(IPCParentFinality {
            height,
            block_hash,
            top_down_msgs: vec![],
            config: Default::default(),
        }))
    }

    pub fn check_finality(&self, other_finality: &IPCParentFinality) -> bool {
        let this_finality = match self.finality_proposal_at_height(other_finality.height) {
            Ok(Some(finality)) => finality,
            _ => {
                tracing::info!("cannot create next finality, check return false");
                return false;
            }
        };

        this_finality == *other_finality
    }
}
