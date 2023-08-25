// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A constant running process that fetch or listener to parent state

use crate::error::Error;
use crate::{BlockHash, BlockHeight, Config, ParentFinalityProvider};
use anyhow::Context;
use async_stm::atomically_or_err;
use fvm_shared::clock::ChainEpoch;
use ipc_agent_sdk::apis::IpcAgentClient;
use ipc_agent_sdk::jsonrpc::JsonRpcClientImpl;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::subnet_id::SubnetID;
use ipc_sdk::ValidatorSet;
use std::sync::Arc;
use std::time::Duration;

/// Constantly syncing with parent through polling
pub struct PollingParentSyncer<T> {
    config: Config,
    parent_view_provider: Arc<T>,
    agent: Arc<IPCAgentProxy>,
}

impl<T> PollingParentSyncer<T> {
    pub fn new(config: Config, parent_view_provider: Arc<T>, agent: Arc<IPCAgentProxy>) -> Self {
        Self {
            config,
            parent_view_provider,
            agent,
        }
    }
}

impl<T: ParentFinalityProvider + Send + Sync + 'static> PollingParentSyncer<T> {
    /// Start the parent finality listener in the background
    pub fn start(self) -> anyhow::Result<()> {
        let config = self.config;
        let provider = self.parent_view_provider;
        let agent = self.agent;

        tokio::spawn(async move {
            loop {
                if let Err(e) = sync_with_parent(&config, &agent, &provider).await {
                    tracing::info!("sync with parent encountered error: {e}");
                }
            }
        });

        Ok(())
    }
}

/// Extract the actual error from Box
macro_rules! downcast_err {
    ($r:expr) => {
        match $r {
            Ok(v) => Ok(v),
            Err(e) => match e.downcast_ref::<Error>() {
                None => unreachable!(),
                Some(e) => Err(e.clone()),
            },
        }
    };
}

async fn sync_with_parent<T: ParentFinalityProvider + Send + Sync + 'static>(
    config: &Config,
    agent_proxy: &Arc<IPCAgentProxy>,
    provider: &Arc<T>,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(config.polling_interval_secs));

    loop {
        interval.tick().await;

        // update block hash and validator set
        let latest_height = agent_proxy
            .get_chain_head_height()
            .await
            .context("cannot fetch parent chain head")?;

        if latest_height < config.chain_head_delay {
            tracing::debug!("latest height not more than the chain head delay");
            continue;
        }

        let r = atomically_or_err(|| {
            Ok(if let Some(h) = provider.latest_height()? {
                h + 1
            } else {
                let last_committed_finality = provider.last_committed_finality()?;
                last_committed_finality.height + 1
            })
        })
        .await;
        let starting_height = downcast_err!(r)?;

        // we are going backwards in terms of block height, the latest block height is lower
        // than our previously fetched head. It could be a chain reorg. We clear all the cache
        // in `provider` and start from scratch
        if starting_height > latest_height {
            // FIXME: the most brutal way is to
            todo!()
        }

        let mut block_height_to_update = vec![];
        for h in starting_height..=latest_height {
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

        let r = atomically_or_err(move || {
            for (height, block_hash, validator_set, messages) in block_height_to_update.clone() {
                let r = provider.new_parent_view(height, block_hash, validator_set, messages);
                match downcast_err!(r) {
                    Ok(_) => {},
                    Err(e) => {}
                }
            }
            Ok(())
        })
        .await;
        downcast_err!(r)?;
    }
}

pub struct IPCAgentProxy {
    agent_client: IpcAgentClient<JsonRpcClientImpl>,
    parent_subnet: SubnetID,
}

impl IPCAgentProxy {
    pub fn new(client: IpcAgentClient<JsonRpcClientImpl>, parent: SubnetID) -> Self {
        Self {
            agent_client: client,
            parent_subnet: parent,
        }
    }

    pub async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
        let head = self
            .agent_client
            .get_chain_head(&self.parent_subnet)
            .await?;
        Ok(head.height)
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
                &self.parent_subnet,
                start_height as ChainEpoch,
                end_height as ChainEpoch,
            )
            .await
    }

    pub async fn get_validator_set(&self, height: BlockHeight) -> anyhow::Result<ValidatorSet> {
        self.agent_client
            .get_validator_set(&self.parent_subnet, Some(height as ChainEpoch))
            .await
    }
}
