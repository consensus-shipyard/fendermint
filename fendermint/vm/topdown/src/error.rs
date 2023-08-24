// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{BlockHeight, Bytes, Nonce, SequentialAppendError};
use thiserror::Error;

/// The errors for top down checkpointing
#[derive(Error, Debug, Eq, PartialEq, Clone)]
pub enum Error {
    #[error("The latest height of the parent view is not ready")]
    HeightNotReady,
    #[error("The latest height has yet to reach the configured threshold")]
    HeightThresholdNotReached,
    #[error("The data specified in this height is not found in cache")]
    HeightNotFoundInCache(BlockHeight),
    #[error("Exceeding current parent view's latest block height.")]
    ExceedingLatestHeight {
        proposal: BlockHeight,
        parent: BlockHeight,
    },
    #[error("The block height in the proposal is already committed")]
    HeightAlreadyCommitted(BlockHeight),
    #[error("Proposal's block hash and parent's block hash not match")]
    BlockHashNotMatch {
        proposal: Bytes,
        parent: Bytes,
        height: BlockHeight,
    },
    #[error("Proposal's block hash at height not found in parent view")]
    BlockHashNotFound(BlockHeight),
    #[error("Proposal's validator set and that of the parent view not match")]
    ValidatorSetNotMatch(BlockHeight),
    #[error("Proposal's validator set at height not found in parent view")]
    ValidatorSetNotFound(BlockHeight),
    #[error("Proposal's min top down msg nonce is smaller than the last committed nonce")]
    InvalidNonce {
        proposal: Nonce,
        parent: Nonce,
        block: BlockHeight,
    },
    #[error("Incoming top down messages are not order by nonce sequentially")]
    NonceNotSequential,
    #[error("The parent view update with block height is not sequential")]
    NonSequentialParentViewInsert(SequentialAppendError),
}
