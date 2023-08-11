// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! IPC messages

use crate::pof::IPCParentFinality;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IPCMessage {
    TopDown(IPCParentFinality),
    BottomUp,
}

impl From<IPCMessage> for Vec<u8> {
    fn from(value: IPCMessage) -> Self {
        serde_json::to_vec(&value).expect("should not happen")
    }
}
