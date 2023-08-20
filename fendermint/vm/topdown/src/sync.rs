// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A constant running process that fetch or listener to parent state


use crate::{BlockHeight, Config, Nonce, ParentFinalityProvider};
use anyhow::anyhow;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use std::cmp::max;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Constantly syncing with parent through polling
pub struct PollingParentSyncer<T> {
    config: Config,
    started: Arc<AtomicBool>,
    parent_view_provider: Arc<T>,
    agent: Arc<IPCAgentProxy>,
}

impl <T> PollingParentSyncer<T> {
    pub fn new(config: Config, parent_view_provider: Arc<T>, agent: Arc<IPCAgentProxy>) -> Self {
        Self {
            config,
            started: Arc::new(AtomicBool::new(false)),
            parent_view_provider,
            agent
        }
    }
}

impl <T: ParentFinalityProvider + Send + Sync + 'static> PollingParentSyncer<T> {
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

        let handle =
            tokio::spawn(async move { sync_with_parent(config, agent, provider).await });
        self.handle = Some(handle);

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

        let
        let hashes = fetch_block_hashes(&config, &agent_proxy, &lock).await?;
        update_top_down_msgs(&config, &agent_proxy, &lock).await?;
        update_membership(&config, &agent_proxy, &lock).await?;

        let mut cache = lock.write().unwrap();
        for r in hashes {
            cache.block_hash.insert_after_lower_bound(r.0, r.1);
        }
    }
}

async fn update_top_down_msgs(
    _config: &Config,
    _agent_proxy: &Arc<AgentProxy>,
    _lock: &LockedCache,
) -> anyhow::Result<Vec<CrossMsg>> {
    Ok(vec![])
}

async fn update_membership(
    _config: &Config,
    _agent_proxy: &Arc<AgentProxy>,
    _lock: &LockedCache,
) -> anyhow::Result<ValidatorSet> {
    Ok(ValidatorSet::default())
}

async fn fetch_block_hashes(
    config: &Config,
    agent_proxy: &Arc<AgentProxy>,
    lock: &LockedCache,
) -> anyhow::Result<Vec<(BlockHeight, Vec<u8>)>> {
    let latest_height = match agent_proxy.get_chain_head_height().await {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!("cannot fetch parent chain head due to {e}");

            // not throw errors, caller will retry
            return Ok(vec![]);
        }
    };

    if latest_height < config.chain_head_delay {
        tracing::debug!("latest height not more than the chain head delay");
        return Ok(vec![]);
    }

    let starting_height = {
        let cache = lock.read().unwrap();
        // if cache.latest_height() is None, it means we have not started fetching any heights
        // we just use the latest height minus a lower bound as the parent view
        let starting_height = cache.block_hash.upper_bound().unwrap_or(max(
            1,
            latest_height.saturating_sub(config.chain_head_lower_bound),
        ));
        tracing::debug!("polling parent from {starting_height} till {latest_height}");

        starting_height

        // ensure cache is dropped
    };

    // FIXME: make the fetching batching and concurrent
    let mut results = vec![];
    for h in starting_height..=latest_height {
        let block_hash = agent_proxy.get_block_hash(h).await?;
        results.push((h, block_hash));
    }

    Ok(results)
}

struct IPCAgentProxy {}

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

    pub async fn get_validator_set(&self) -> anyhow::Result<ValidatorSet> {
        todo!()
    }
}