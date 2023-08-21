// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A constant running process that fetch or listener to parent state

use crate::error::Error;
use crate::{BlockHeight, Config, ParentFinalityProvider};
use anyhow::anyhow;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use std::cmp::max;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Constantly syncing with parent through polling
pub struct PollingParentSyncer<T> {
    config: Config,
    started: Arc<AtomicBool>,
    parent_view_provider: Arc<T>,
    agent: Arc<IPCAgentProxy>,
}

impl<T> PollingParentSyncer<T> {
    pub fn new(config: Config, parent_view_provider: Arc<T>, agent: Arc<IPCAgentProxy>) -> Self {
        Self {
            config,
            started: Arc::new(AtomicBool::new(false)),
            parent_view_provider,
            agent,
        }
    }
}

impl<T: ParentFinalityProvider + Send + Sync + 'static> PollingParentSyncer<T> {
    /// Start the proof of finality listener in the background
    pub fn start(&mut self) -> anyhow::Result<()> {
        match self
            .started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {}
            Err(_) => return Err(anyhow!("already started")),
        }

        let config = self.config.clone();
        let provider = self.parent_view_provider.clone();
        let agent = self.agent.clone();

        tokio::spawn(async move {
            if let Err(e) = sync_with_parent(config, agent, provider).await {
                tracing::info!("sync with parent encountered error: {e}");
            }
        });

        Ok(())
    }
}

async fn sync_with_parent<T: ParentFinalityProvider + Send + Sync + 'static>(
    config: Config,
    agent_proxy: Arc<IPCAgentProxy>,
    provider: Arc<T>,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(config.polling_interval));

    loop {
        interval.tick().await;

        // update block hash and validator set
        let latest_height = match agent_proxy.get_chain_head_height().await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("cannot fetch parent chain head due to {e}");

                // not throw errors, caller will retry
                continue;
            }
        };

        if latest_height < config.chain_head_delay {
            tracing::debug!("latest height not more than the chain head delay");
            continue;
        }

        let starting_height = if let Some(h) = provider.latest_height().await {
            h + 1
        } else {
            // if latest_recorded_height is None, it means we have not started fetching any heights
            // we just use the latest height minus a lower bound as the parent view
            max(
                1,
                latest_height.saturating_sub(config.chain_head_lower_bound),
            )
        };

        for h in starting_height..=latest_height {
            let block_hash = agent_proxy.get_block_hash(h).await?;
            let validator_set = agent_proxy.get_validator_set(h).await?;

            match provider
                .new_parent_view(Some((h, block_hash, validator_set)), vec![])
                .await
            {
                Ok(_) => {}
                Err(e) => match e {
                    Error::ParentReorgDetected(_) => {
                        todo!()
                    }
                    _ => unreachable!(),
                },
            }
        }

        // now get the top down messages
        let nonce = provider.latest_nonce().await.unwrap_or(0);
        let top_down_msgs = agent_proxy.get_top_down_msgs(latest_height, nonce).await?;
        match provider.new_parent_view(None, top_down_msgs).await {
            Ok(_) => {}
            Err(_) => {
                todo!()
            }
        }
    }
}

pub struct IPCAgentProxy {}

impl IPCAgentProxy {
    pub async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
        todo!()
    }

    pub async fn get_block_hash(&self, _height: BlockHeight) -> anyhow::Result<Vec<u8>> {
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
