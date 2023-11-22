// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! The inner type of parent syncer

use crate::finality::ParentViewPayload;
use crate::proxy::{IPCProviderProxy, ParentQueryProxy};
use crate::sync::pointers::SyncPointers;
use crate::sync::{query_starting_finality, ParentFinalityStateQuery};
use crate::{
    is_null_round_str, BlockHash, BlockHeight, CachedFinalityProvider, Config, Error, Toggle,
};
use anyhow::{anyhow, Context};
use async_stm::{atomically, atomically_or_err};
use ethers::utils::hex;
use std::sync::Arc;

/// The parent syncer that constantly poll parent. This struct handles lotus null blocks and delayed
/// execution. For ETH based parent, it should work out of the box as well.
pub(crate) struct LotusParentSyncer<T, C> {
    config: Config,
    parent_proxy: Arc<IPCProviderProxy>,
    provider: Arc<Toggle<CachedFinalityProvider<IPCProviderProxy>>>,
    query: Arc<T>,
    tendermint_client: C,

    /// The pointers that indicate which height to poll parent next
    sync_pointers: SyncPointers,
}

impl<T, C> LotusParentSyncer<T, C>
where
    T: ParentFinalityStateQuery + Send + Sync + 'static,
    C: tendermint_rpc::Client + Send + Sync + 'static,
{
    pub async fn new(
        config: Config,
        parent_proxy: Arc<IPCProviderProxy>,
        provider: Arc<Toggle<CachedFinalityProvider<IPCProviderProxy>>>,
        query: Arc<T>,
        tendermint_client: C,
    ) -> anyhow::Result<Self> {
        let last_committed_finality = atomically(|| provider.last_committed_finality())
            .await
            .ok_or_else(|| anyhow!("parent finality not ready"))?;

        Ok(Self {
            config,
            parent_proxy,
            provider,
            query,
            tendermint_client,
            sync_pointers: SyncPointers::new(last_committed_finality.height),
        })
    }

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
    /// `tail = 2`, `to_confirm = Some(4)`.
    /// The data fetch at block height 2 is pushed to cache and height 2 is ready to be proposed.
    ///
    /// At height 6, it's block height 4 will be confirmed and its data pushed to cache. At the same
    /// time, since block 3 is a null block, empty data will also be pushed to cache. Block 4 is ready
    /// to be proposed.
    pub async fn sync(&mut self) -> anyhow::Result<()> {
        if self.is_syncing_peer().await? {
            tracing::debug!("syncing with peer, skip parent finality syncing this round");
            return Ok(());
        }

        let chain_head = if let Some(h) = self.finalized_chain_head().await? {
            h
        } else {
            return Ok(());
        };
        tracing::debug!(
            chain_head,
            pointers = self.sync_pointers.to_string(),
            "syncing heights"
        );

        if self.detected_reorg_by_height(chain_head) {
            tracing::warn!(
                pointers = self.sync_pointers.to_string(),
                chain_head,
                "reorg detected from height"
            );
            return self.reset_cache().await;
        }

        if !self.has_new_blocks(chain_head) {
            tracing::debug!("the parent has yet to produce a new block");
            return Ok(());
        }

        let tail = self.sync_pointers.tail();
        if let Some((confirmed_height, payload)) = self.poll_next().await? {
            atomically_or_err::<_, Error, _>(|| {
                for h in (tail + 1)..confirmed_height {
                    self.provider.new_parent_view(h, None)?;
                    tracing::debug!(height = h, "null block pushed to cache");
                }
                self.provider
                    .new_parent_view(confirmed_height, Some(payload.clone()))?;
                tracing::debug!(height = confirmed_height, "non-null block pushed to cache");
                Ok(())
            })
            .await?;
        }

        Ok(())
    }
}

impl<T, C> LotusParentSyncer<T, C>
where
    T: ParentFinalityStateQuery + Send + Sync + 'static,
    C: tendermint_rpc::Client + Send + Sync + 'static,
{
    async fn is_syncing_peer(&self) -> anyhow::Result<bool> {
        let status: tendermint_rpc::endpoint::status::Response = self
            .tendermint_client
            .status()
            .await
            .context("failed to get Tendermint status")?;
        Ok(status.sync_info.catching_up)
    }

    /// Poll the next block height. Returns finalized and executed block data.
    async fn poll_next(&mut self) -> Result<Option<(BlockHeight, ParentViewPayload)>, Error> {
        let height = self.sync_pointers.head() + 1;
        let parent_block_hash = self.non_null_parent_hash().await;

        let block_hash_res = match self.parent_proxy.get_block_hash(height).await {
            Ok(res) => res,
            Err(e) => {
                let err = e.to_string();
                if is_null_round_str(&err) {
                    tracing::warn!(height, "null round at height");

                    self.sync_pointers.advance_head();

                    return Ok(None);
                }
                return Err(Error::CannotQueryParent(err, height));
            }
        };

        if block_hash_res.parent_block_hash != parent_block_hash {
            tracing::warn!(
                height,
                parent_hash = hex::encode(&block_hash_res.parent_block_hash),
                previous_hash = hex::encode(&parent_block_hash),
                "parent block hash diff than previous hash",
            );
            return Err(Error::ParentChainReorgDetected);
        }

        if let Some(to_confirm) = self.sync_pointers.to_confirm() {
            tracing::warn!(
                height,
                confirm = to_confirm,
                "non-null round at height, confirmed previous height"
            );
            let data = self
                .fetch_data(to_confirm, block_hash_res.block_hash)
                .await?;
            self.sync_pointers.advance_confirm(height);
            return Ok(Some((to_confirm, data)));
        }

        tracing::warn!(height, "non-null round at height, waiting for confirmation");
        self.sync_pointers.advance_head();

        Ok(None)
    }

    async fn fetch_data(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
    ) -> Result<ParentViewPayload, Error> {
        let changes_res = self
            .parent_proxy
            .get_validator_changes(height)
            .await
            .map_err(|e| Error::CannotQueryParent(e.to_string(), height))?;
        if changes_res.block_hash != block_hash {
            tracing::warn!(
                height,
                change_set_hash = hex::encode(&changes_res.block_hash),
                block_hash = hex::encode(&block_hash),
                "change set block hash does not equal block hash",
            );
            return Err(Error::ParentChainReorgDetected);
        }

        let top_down_msgs_res = self
            .parent_proxy
            .get_top_down_msgs_with_hash(height, &block_hash)
            .await
            .map_err(|e| Error::CannotQueryParent(e.to_string(), height))?;

        Ok((block_hash, changes_res.value, top_down_msgs_res))
    }

    /// We only want the non-null parent block's hash
    async fn non_null_parent_hash(&self) -> BlockHash {
        let parent_height = match (self.sync_pointers.to_confirm(), self.sync_pointers.tail()) {
            (Some(height), _) => {
                tracing::debug!(height, "found height to confirm");
                height
            }
            (None, height) => {
                tracing::debug!(height, "no height to confirm");
                height
            }
        };
        match atomically(|| self.provider.block_hash(parent_height)).await {
            Some(hash) => hash,
            None => unreachable!("guaranteed to have block hash at height {}", parent_height),
        }
    }

    fn has_new_blocks(&self, height: BlockHeight) -> bool {
        self.sync_pointers.head() < height
    }

    fn detected_reorg_by_height(&self, height: BlockHeight) -> bool {
        // If the below is true, we are going backwards in terms of block height, the latest block
        // height is lower than our previously fetched head. It could be a chain reorg.
        self.sync_pointers.head() > height
    }

    async fn finalized_chain_head(&self) -> anyhow::Result<Option<BlockHeight>> {
        let parent_chain_head_height = self.parent_proxy.get_chain_head_height().await?;
        // sanity check
        if parent_chain_head_height < self.config.chain_head_delay {
            tracing::debug!("latest height not more than the chain head delay");
            return Ok(None);
        }

        // we consider the chain head finalized only after the `chain_head_delay`
        Ok(Some(
            parent_chain_head_height - self.config.chain_head_delay,
        ))
    }

    /// Reset the cache in the face of a reorg
    async fn reset_cache(&self) -> anyhow::Result<()> {
        let finality = query_starting_finality(&self.query, &self.parent_proxy).await?;
        atomically(|| self.provider.reset(finality.clone())).await;
        Ok(())
    }
}
