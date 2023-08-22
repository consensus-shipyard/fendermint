// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod cache;

use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use serde::{Deserialize, Serialize};

pub use crate::cache::{SequentialAppendError, SequentialKeyCache, ValueIter};

type BlockHeight = u64;

/// The finality view for IPC parent at certain height.
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
