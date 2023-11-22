// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::BlockHeight;
use std::fmt::{Display, Formatter};

/// There are three pointers, each refers to a block height, when syncing with parent. As Lotus has
/// delayed execution and null round, we need to ensure the topdown messages and validator
/// changes polled are indeed finalized and executed. The following three pointers are introduced:
///     - tail: The latest block height in cache that is finalized and executed
///     - to_confirm: The next block height in cache to be confirmed executed, could be None
///     - head: The latest block height fetched in cache, finalized but may not be executed.
///
/// Say we have block chain as follows:
/// NonNullBlock(1) -> NonNullBlock(2) -> NullBlock(3) -> NonNullBlock(4) -> NullBlock(5) -> NonNullBlock(6)
/// and block height 1 is the previously finalized and executed block height.
///
/// At the beginning, tail == head == 1 and to_confirm == None. With a new block height fetched,
/// `head = 2`. Since height at 2 is not a null block, `to_confirm = Some(2)`, because we cannot be sure
/// block 2 has executed yet. When a new block is fetched, `head = 3`. Since head is a null block, we
/// cannot confirm block height 2. When `head = 4`, it's not a null block, we can confirm block 2 is
/// executed (also with some checks to ensure no reorg has occurred). We fetch block 2's data and set
/// `tail = 2`, `to_confirm = Some(4)`. Then height 2 is ready to be proposed.
#[derive(Clone, Debug)]
pub(crate) struct SyncPointers {
    tail: BlockHeight,
    to_confirm: Option<BlockHeight>,
    head: BlockHeight,
}

impl SyncPointers {
    pub fn new(tail: BlockHeight) -> Self {
        Self {
            tail,
            to_confirm: None,
            head: tail,
        }
    }

    pub fn head(&self) -> BlockHeight {
        self.head
    }

    pub fn to_confirm(&self) -> Option<BlockHeight> {
        self.to_confirm
    }

    pub fn tail(&self) -> BlockHeight {
        self.tail
    }

    pub fn advance_head(&mut self) {
        self.head += 1;
    }

    pub fn advance_confirm(&mut self, height: BlockHeight) {
        if let Some(h) = self.to_confirm {
            self.tail = h;
        }
        self.to_confirm = Some(height);
    }
}

impl Display for SyncPointers {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{tail: {}, to_confirm: {:?}, head: {}}}",
            self.tail, self.to_confirm, self.head
        )
    }
}
