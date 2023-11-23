// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{BlockHash, BlockHeight};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub(crate) struct SyncPointers {
    tail: BlockHeight,
    to_confirm: Option<(BlockHeight, BlockHash)>,
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

    pub fn to_confirm(&self) -> Option<(BlockHeight, BlockHash)> {
        self.to_confirm.clone()
    }

    pub fn tail(&self) -> BlockHeight {
        self.tail
    }

    pub fn advance_head(&mut self) {
        self.head += 1;
    }

    pub fn set_tail(&mut self, height: BlockHeight) {
        self.tail = height;
    }

    pub fn set_confirmed(&mut self, height: BlockHeight, hash: BlockHash) {
        self.to_confirm = Some((height, hash));
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
