// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{BlockHash, BlockHeight, SequentialAppendError};
use thiserror::Error;

/// The errors for top down checkpointing
#[derive(Error, Debug, Eq, PartialEq, Clone)]
pub enum Error {
    #[error("The committed finality is not ready for query yet")]
    CommittedFinalityNotReady,
    #[error("The latest height of the parent view is not ready")]
    HeightNotReady,
    #[error("The data specified in this height is not found in cache")]
    HeightNotFoundInCache(BlockHeight),
    #[error("Exceeding current parent view's latest block height")]
    ExceedingLatestHeight {
        proposal: BlockHeight,
        parent: BlockHeight,
    },
    #[error("The block height in the proposal is already committed")]
    HeightAlreadyCommitted(BlockHeight),
    #[error("Proposal's block hash and parent's block hash not match")]
    BlockHashNotMatch {
        proposal: BlockHash,
        parent: BlockHash,
        height: BlockHeight,
    },
    #[error("Proposal's block hash at height not found in parent view")]
    BlockHashNotFound(BlockHeight),
    #[error("Incoming top down messages are not order by nonce sequentially")]
    NonceNotSequential,
    #[error("The parent view update with block height is not sequential")]
    NonSequentialParentViewInsert(SequentialAppendError),
}
