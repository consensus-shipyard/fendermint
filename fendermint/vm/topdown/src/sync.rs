// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A constant running process that fetch or listener to parent state

use crate::error::Error;
use crate::{BlockHash, BlockHeight, Config, IPCParentFinality, ParentFinalityProvider};
use anyhow::{anyhow, Context};
use async_stm::atomically_or_err;
use fvm_shared::clock::ChainEpoch;
use ipc_agent_sdk::apis::IpcAgentClient;
use ipc_agent_sdk::jsonrpc::JsonRpcClientImpl;
use ipc_agent_sdk::message::ipc::ValidatorSet;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::subnet_id::SubnetID;
use std::sync::Arc;
use std::time::Duration;

/// Query the parent finality from the block chain state
pub trait ParentFinalityStateQuery {
    /// Get the latest committed finality from the state
    fn get_latest_committed_finality(&self) -> anyhow::Result<Option<IPCParentFinality>>;
}

/// Constantly syncing with parent through polling
pub struct PollingParentSyncer<P> {
    config: Config,
    parent_view_provider: Arc<P>,
    agent: Arc<IPCAgentProxy>,
}

/// Start the polling parent syncer in the background
pub async fn launch_polling_syncer<
    Q: ParentFinalityStateQuery + Send + Sync + 'static,
    P: ParentFinalityProvider + Send + Sync + 'static,
>(
    query: &Q,
    config: Config,
    view_provider: Arc<P>,
    agent: IPCAgentProxy,
) -> anyhow::Result<()> {
    loop {
        let finality = match query.get_latest_committed_finality() {
            Ok(Some(finality)) => finality,
            Ok(None) => {
                tracing::debug!("app not ready for query yet");
                continue;
            }
            Err(e) => {
                tracing::warn!("cannot get committed finality: {e}");
                continue;
            }
        };

        atomically_or_err(|| view_provider.set_new_finality(finality.clone())).await?;

        let poll = PollingParentSyncer::new(config, view_provider, Arc::new(agent));
        poll.start();

        return Ok(());
    }
}

impl<P> PollingParentSyncer<P> {
    pub fn new(config: Config, parent_view_provider: Arc<P>, agent: Arc<IPCAgentProxy>) -> Self {
        Self {
            config,
            parent_view_provider,
            agent,
        }
    }
}

impl<P: ParentFinalityProvider + Send + Sync + 'static> PollingParentSyncer<P> {
    /// Start the parent finality listener in the background
    pub fn start(self) {
        let config = self.config;
        let provider = self.parent_view_provider;
        let agent = self.agent;

        tokio::spawn(async move {
            loop {
                // Syncing with parent with the below steps:
                // 1. Get the latest height in cache or latest height committed increment by 1 as the
                //    starting height
                // 2. Get the latest chain head height deduct away N blocks as the ending height
                // 3. Fetches the data between starting and ending height
                // 4. Update the data into cache
                if let Err(e) = sync_with_parent(&config, &agent, &provider).await {
                    tracing::error!("sync with parent encountered error: {e}");
                }
            }
        });
    }
}

async fn sync_with_parent<T: ParentFinalityProvider + Send + Sync + 'static>(
    config: &Config,
    agent_proxy: &Arc<IPCAgentProxy>,
    provider: &Arc<T>,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(config.polling_interval_secs));

    loop {
        interval.tick().await;

        let starting_height = get_starting_height(provider).await?;
        let latest_height = agent_proxy
            .get_chain_head_height()
            .await
            .context("cannot fetch parent chain head")?;
        if latest_height < config.chain_head_delay {
            tracing::debug!("latest height not more than the chain head delay");
            continue;
        }
        let ending_height = latest_height - config.chain_head_delay;

        tracing::debug!("starting height: {starting_height}, ending_height: {ending_height}");

        // we are going backwards in terms of block height, the latest block height is lower
        // than our previously fetched head. It could be a chain reorg. We clear all the cache
        // in `provider` and start from scratch
        if starting_height > ending_height {
            todo!()
        }

        let new_parent_views =
            get_new_parent_views(agent_proxy, starting_height, ending_height).await?;
        tracing::debug!("new parent views: {new_parent_views:?}");

        atomically_or_err::<_, Error, _>(move || {
            for (height, block_hash, validator_set, messages) in new_parent_views.clone() {
                provider.new_parent_view(height, block_hash, validator_set, messages)?;
            }
            Ok(())
        })
        .await?;

        tracing::debug!("updated new parent views till height: {ending_height}");
    }
}

/// Obtain the starting block height to perform the parent view update
async fn get_starting_height<T: ParentFinalityProvider + Send + Sync + 'static>(
    provider: &Arc<T>,
) -> anyhow::Result<BlockHeight> {
    let starting_height = atomically_or_err::<_, Error, _>(|| {
        // we are adding 1 to the height because we are fetching block by block, we also configured
        // the sequential cache to use increment == 1.
        Ok(if let Some(h) = provider.latest_height()? {
            h + 1
        } else {
            let last_committed_finality = provider.last_committed_finality()?;
            last_committed_finality.height + 1
        })
    })
    .await?;

    Ok(starting_height)
}

/// Obtain the new parent views for the input block height range
async fn get_new_parent_views(
    agent_proxy: &Arc<IPCAgentProxy>,
    start_height: BlockHeight,
    end_height: BlockHeight,
) -> anyhow::Result<Vec<(BlockHeight, BlockHash, ValidatorSet, Vec<CrossMsg>)>> {
    let mut block_height_to_update = vec![];
    for h in start_height..=end_height {
        let block_hash = agent_proxy
            .get_block_hash(h)
            .await
            .context("cannot fetch block hash")?;
        let validator_set = agent_proxy
            .get_validator_set(h)
            .await
            .context("cannot fetch validator set")?;
        let top_down_msgs = agent_proxy
            .get_top_down_msgs(h, h)
            .await
            .context("cannot fetch top down messages")?;

        block_height_to_update.push((h, block_hash, validator_set, top_down_msgs));
    }
    Ok(block_height_to_update)
}

pub struct IPCAgentProxy {
    agent_client: IpcAgentClient<JsonRpcClientImpl>,
    parent_subnet: SubnetID,
    child_subnet: SubnetID,
}

impl IPCAgentProxy {
    pub fn new(
        client: IpcAgentClient<JsonRpcClientImpl>,
        target_subnet: SubnetID,
    ) -> anyhow::Result<Self> {
        let parent = target_subnet
            .parent()
            .ok_or_else(|| anyhow!("subnet does not have parent"))?;
        Ok(Self {
            agent_client: client,
            parent_subnet: parent,
            child_subnet: target_subnet,
        })
    }

    pub async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
        let height = self
            .agent_client
            .get_chain_head_height(&self.parent_subnet)
            .await?;
        Ok(height as BlockHeight)
    }

    pub async fn get_block_hash(&self, height: BlockHeight) -> anyhow::Result<BlockHash> {
        self.agent_client
            .get_block_hash(&self.parent_subnet, height as ChainEpoch)
            .await
    }

    pub async fn get_top_down_msgs(
        &self,
        start_height: BlockHeight,
        end_height: u64,
    ) -> anyhow::Result<Vec<CrossMsg>> {
        self.agent_client
            .get_top_down_msgs(
                &self.child_subnet,
                start_height as ChainEpoch,
                end_height as ChainEpoch,
            )
            .await
    }

    pub async fn get_validator_set(&self, height: BlockHeight) -> anyhow::Result<ValidatorSet> {
        let r = self
            .agent_client
            .get_validator_set(&self.child_subnet, Some(height as ChainEpoch))
            .await?;
        Ok(r.validator_set)
    }
}
