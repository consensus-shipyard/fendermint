// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A constant running process that fetch or listener to parent state

use crate::error::Error;
use crate::{BlockHash, BlockHeight, Config, ParentFinalityProvider};
use anyhow::Context;
use async_stm::atomically_or_err;
use ipc_sdk::cross::CrossMsg;
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
        let config = self.config.clone();
        let provider = self.parent_view_provider.clone();
        let agent = self.agent.clone();

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
                h + config.block_interval
            } else {
                let last_committed_finality = provider.last_committed_finality()?;
                last_committed_finality.height + config.block_interval
            })
        })
        .await;
        let starting_height = downcast_err!(r)?;

        if starting_height > latest_height {
            // FIXME: the most brutal way is to clear all the cache in `provider` and
            // FIXME: start from scratch.
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

            block_height_to_update.push((h, block_hash, validator_set));
        }

        let r = atomically_or_err(|| {
            for (height, block_hash, validator_set) in block_height_to_update {
                provider.new_block_height(height, block_hash, validator_set)?;
            }
            Ok(())
        })
        .await;
        downcast_err!(r)?;

        // now get the top down messages
        let nonce = downcast_err!(
            atomically_or_err(|| {
                let nonce = if let Some(nonce) = provider.latest_nonce()? {
                    nonce
                } else {
                    let finality = provider.last_committed_finality()?;
                    if finality.top_down_msgs.is_empty() {
                        0
                    } else {
                        finality.top_down_msgs.last().unwrap().msg.nonce
                    }
                };

                Ok(nonce + 1)
            })
            .await
        )?;

        let top_down_msgs = agent_proxy.get_top_down_msgs(latest_height, nonce).await?;
        // safe to unwrap as updating top down msgs will not trigger error
        downcast_err!(atomically_or_err(|| provider.new_top_down_msgs(top_down_msgs)).await);
    }
}

pub struct IPCAgentProxy {
    agent_client:
}

impl IPCAgentProxy {
    pub async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
        todo!()
    }

    pub async fn get_block_hash(&self, _height: BlockHeight) -> anyhow::Result<BlockHash> {
        todo!()
    }

    pub async fn get_top_down_msgs(
        &self,
        _height: BlockHeight,
        _nonce: u64,
    ) -> anyhow::Result<Vec<CrossMsg>> {
        todo!()
    }

    pub async fn get_validator_set(&self, _height: BlockHeight) -> anyhow::Result<ValidatorSet> {
        todo!()
    }
}
