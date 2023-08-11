// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use crate::BlockHeight;

/// The ipc agent proxy. Use this struct to interface with the running ipc-agent.
pub(crate) struct AgentProxy {}

impl AgentProxy {
    pub async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
        todo!()
    }

    pub async fn get_block_hash(&self, _height: BlockHeight) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    pub async fn get_top_down_msgs(&self, _height: BlockHeight, _nonce: u64) -> anyhow::Result<Vec<CrossMsg>> {
        todo!()
    }

    pub async fn get_membership(&self) -> anyhow::Result<ValidatorSet> {
        todo!()
    }
}
