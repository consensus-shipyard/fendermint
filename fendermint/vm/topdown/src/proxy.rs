// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{BlockHash, BlockHeight};
use anyhow::anyhow;
use async_trait::async_trait;
use fvm_shared::clock::ChainEpoch;
use ipc_provider::manager::{GetBlockHashResult, TopDownQueryPayload};
use ipc_provider::IpcProvider;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::staking::StakingChangeRequest;
use ipc_sdk::subnet_id::SubnetID;
use std::future::Future;

/// The null round error message
const NULL_ROUND_ERR_MSG: &str = "requested epoch was a null round";
/// The default block hash for null round
const NULL_BLOCK_HASH: Vec<u8> = vec![];

/// The interface to querying state of the parent
#[async_trait]
pub trait ParentQueryProxy {
    /// Get the parent chain head block number or block height
    async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight>;

    /// Get the genesis epoch of the child subnet, i.e. the epoch that the subnet was created in
    /// the parent subnet.
    async fn get_genesis_epoch(&self) -> anyhow::Result<BlockHeight>;

    /// Getting the block hash at the target height.
    async fn get_block_hash(&self, height: BlockHeight) -> anyhow::Result<GetBlockHashResult>;

    /// Get the top down messages at epoch with the block hash at that height
    async fn get_top_down_msgs_with_hash(
        &self,
        height: BlockHeight,
        block_hash: &BlockHash,
    ) -> anyhow::Result<Vec<CrossMsg>>;

    /// Get the validator set at the specified height
    async fn get_validator_changes(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<TopDownQueryPayload<Vec<StakingChangeRequest>>>;
}

/// The proxy to the subnet's parent
pub struct IPCProviderProxy {
    ipc_provider: IpcProvider,
    /// The parent subnet for the child subnet we are target. We can derive from child subnet,
    /// but storing it separately so that we dont have to derive every time.
    parent_subnet: SubnetID,
    /// The child subnet that this node belongs to.
    child_subnet: SubnetID,
}

impl IPCProviderProxy {
    pub fn new(ipc_provider: IpcProvider, target_subnet: SubnetID) -> anyhow::Result<Self> {
        let parent = target_subnet
            .parent()
            .ok_or_else(|| anyhow!("subnet does not have parent"))?;
        Ok(Self {
            ipc_provider,
            parent_subnet: parent,
            child_subnet: target_subnet,
        })
    }

    /// Handles lotus null round error. If `res` is indeed a null round error, f will be called to
    /// generate the default value.
    ///
    /// This is the error that we see when there is a null round:
    /// https://github.com/filecoin-project/lotus/blob/7bb1f98ac6f5a6da2cc79afc26d8cd9fe323eb30/node/impl/full/eth.go#L164
    /// This happens when we request the block for a round without blocks in the tipset.
    /// A null round will never have a block, which means that we can advance to the next height.
    async fn handle_null_round<T, Fut, F>(&self, res: anyhow::Result<T>, f: F) -> anyhow::Result<T>
    where
        Fut: Future<Output = anyhow::Result<T>> + Send,
        F: Fn() -> Fut,
    {
        match res {
            Ok(v) => Ok(v),
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains(NULL_ROUND_ERR_MSG) {
                    f().await
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[async_trait]
impl ParentQueryProxy for IPCProviderProxy {
    async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
        let height = self.ipc_provider.chain_head(&self.parent_subnet).await?;
        Ok(height as BlockHeight)
    }

    /// Get the genesis epoch of the child subnet, i.e. the epoch that the subnet was created in
    /// the parent subnet.
    async fn get_genesis_epoch(&self) -> anyhow::Result<BlockHeight> {
        let height = self.ipc_provider.genesis_epoch(&self.child_subnet).await?;
        Ok(height as BlockHeight)
    }

    /// Getting the block hash at the target height.
    async fn get_block_hash(&self, height: BlockHeight) -> anyhow::Result<GetBlockHashResult> {
        let r = self
            .ipc_provider
            .get_block_hash(&self.parent_subnet, height as ChainEpoch)
            .await;

        self.handle_null_round(r, || async {
            tracing::warn!("null round detected at height: {height} to get block hash.");

            // Height 1 is null round, we cannot query its parent block anymore, just return
            if height == 1 {
                return Ok(GetBlockHashResult {
                    parent_block_hash: NULL_BLOCK_HASH,
                    block_hash: NULL_BLOCK_HASH,
                });
            }

            let prev_r = self
                .ipc_provider
                .get_block_hash(&self.parent_subnet, height as ChainEpoch - 1)
                .await
                .map(|v| v.block_hash);
            let parent_block_hash = self
                .handle_null_round(prev_r, || async { Ok(NULL_BLOCK_HASH) })
                .await?;

            Ok(GetBlockHashResult {
                parent_block_hash,
                block_hash: NULL_BLOCK_HASH,
            })
        })
        .await
    }

    /// Get the top down messages from the starting to the ending height.
    async fn get_top_down_msgs_with_hash(
        &self,
        height: BlockHeight,
        block_hash: &BlockHash,
    ) -> anyhow::Result<Vec<CrossMsg>> {
        let r = self
            .ipc_provider
            .get_top_down_msgs(&self.child_subnet, height as ChainEpoch, block_hash)
            .await;
        self.handle_null_round(r, || async {
            tracing::warn!("null round detected at height: {height} to get top down messages.");
            Ok(vec![])
        }).await
    }

    /// Get the validator set at the specified height.
    async fn get_validator_changes(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<TopDownQueryPayload<Vec<StakingChangeRequest>>> {
        let r = self
            .ipc_provider
            .get_validator_changeset(&self.child_subnet, height as ChainEpoch)
            .await
            .map(|mut v| {
                // sort ascending, we dont assume the changes are ordered
                v.value
                    .sort_by(|a, b| a.configuration_number.cmp(&b.configuration_number));
                v
            });

        self.handle_null_round(r, || async {
            tracing::warn!("null round detected at height: {height} to get validator changes.");
            Ok(TopDownQueryPayload {
                value: vec![],
                block_hash: NULL_BLOCK_HASH,
            })
        })
        .await
    }
}
