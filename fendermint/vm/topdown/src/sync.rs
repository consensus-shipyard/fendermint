// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A constant running process that fetch or listener to parent state


use crate::{BlockHeight, Config, Nonce};
use anyhow::anyhow;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use std::cmp::max;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Constantly syncing with parent through polling
pub struct PollingParentSyncer {
    config: Config,
    started: Arc<AtomicBool>,
    cache: LockedCache,
}

impl PollingParentSyncer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            started: Arc::new(AtomicBool::new(false)),
            cache: Arc::new(RwLock::new(ParentSyncerCache {
                block_hash: RangeKeyCache::new(),
                top_down_message: RangeKeyCache::new(),
                membership: None,
            })),
            ipc_agent_proxy: Arc::new(AgentProxy {}),
            handle: None,
        }
    }
}

impl ParentViewProvider for PollingParentSyncer {
    fn latest_height(&self) -> Option<BlockHeight> {
        self.read_cache(|cache| cache.block_hash.upper_bound())
    }

    fn block_hash(&self, height: BlockHeight) -> Option<Vec<u8>> {
        self.read_cache(|cache| cache.block_hash.get_value(height).map(|v| v.to_vec()))
    }

    fn top_down_msgs(&self, _height: BlockHeight, nonce: Nonce) -> Vec<CrossMsg> {
        self.read_cache(|cache| {
            let v = cache.top_down_message.values_within_range(nonce, None);
            // FIXME: avoid clone here, return references, let caller clone on demand
            v.into_iter().cloned().collect()
        })
    }

    fn membership(&self) -> Option<ValidatorSet> {
        self.read_cache(|cache| cache.membership.clone())
    }

    fn on_finality_committed(&self, finality: &IPCParentFinality) {
        let mut cache = self.cache.write().unwrap();
        cache.block_hash.remove_key_till(finality.height);
    }
}

impl Clone for PollingParentSyncer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            started: Arc::new(AtomicBool::new(false)),
            cache: self.cache.clone(),
            ipc_agent_proxy: self.ipc_agent_proxy.clone(),
            handle: None,
        }
    }
}

struct ParentSyncerCache {
    block_hash: RangeKeyCache<BlockHeight, Vec<u8>>,
    top_down_message: RangeKeyCache<Nonce, CrossMsg>,
    membership: Option<ValidatorSet>,
}

type LockedCache = Arc<RwLock<ParentSyncerCache>>;

impl PollingParentSyncer {
    fn read_cache<F, T>(&self, f: F) -> T
        where
            F: Fn(&ParentSyncerCache) -> T,
    {
        let cache = self.cache.read().unwrap();
        f(&cache)
    }

    /// Start the proof of finality listener in the background
    pub fn start(&mut self) -> anyhow::Result<()> {
        match self
            .started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {}
            Err(_) => return Err(anyhow!("already started")),
        }

        let parent_syncer = self.ipc_agent_proxy.clone();
        let config = self.config.clone();
        let cache = self.cache.clone();

        let handle =
            tokio::spawn(async move { sync_with_parent(config, parent_syncer, cache).await });
        self.handle = Some(handle);

        Ok(())
    }
}

async fn sync_with_parent(
    config: Config,
    agent_proxy: Arc<AgentProxy>,
    lock: LockedCache,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(config.polling_interval));

    loop {
        interval.tick().await;

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