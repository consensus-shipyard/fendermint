// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{BlockHeight, Bytes, Nonce};

/// The errors for top down checkpointing
pub enum Error {
    /// The latest height of the parent view is not ready
    HeightNotReady,
    HeightThresholdNotReached,
    /// The data specified in this height is not found in cache
    HeightNotFoundInCache(BlockHeight),
    /// Exceeding current parent view's latest block height.
    ExceedingLatestHeight {
        proposal: BlockHeight,
        parent: BlockHeight,
    },
    /// The proposal's height is not greater than last committed
    HeightTooLow {
        incoming: BlockHeight,
        parent: BlockHeight,
    },
    /// The block height in the proposal is already committed
    HeightAlreadyCommitted(BlockHeight),
    /// Proposal's block hash and parent's block hash not match
    BlockHashNotMatch {
        proposal: Bytes,
        parent: Bytes,
        height: BlockHeight,
    },
    /// Proposal's block hash at height not found in parent view
    BlockHashNotFound(BlockHeight),
    /// Proposal's validator set and that of the parent view not match
    ValidatorSetNotMatch(BlockHeight),
    /// Proposal's validator set at height not found in parent view
    ValidatorSetNotFound(BlockHeight),
    /// Proposal's min top down msg nonce is smaller than the last committed nonce
    InvalidNonce {
        proposal: Nonce,
        parent: Nonce,
        block: BlockHeight,
    },
    /// Parent block chain reorg detected
    ParentReorgDetected(BlockHeight),
    /// Incoming top down messages are not order by nonce sequentially
    NonceNotSequential,
}
