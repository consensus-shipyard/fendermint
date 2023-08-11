// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Parent view related functions

use crate::agent::AgentProxy;
use crate::pof::IPCParentFinality;
use crate::{BlockHeight, Config, ParentViewProvider};
use anyhow::anyhow;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use std::cmp::{max, min};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use num_traits::{Num, PrimInt};
use tokio::task::JoinHandle;

/// Constantly syncing with parent through polling
pub struct PollingParentSyncer {
    config: Config,
    started: Arc<AtomicBool>,
    cache: LockedCache,
    ipc_agent_proxy: Arc<AgentProxy>,
    handle: Option<JoinHandle<anyhow::Result<()>>>
}

impl ParentViewProvider for PollingParentSyncer {
    fn latest_height(&self) -> BlockHeight {
        self.read_cache(|cache| cache.latest_key())
    }

    fn block_hash(&self, height: u64) -> Option<Vec<u8>> {
        self.read_cache(|cache| cache.get_value(height).map(|v| v.to_vec()))
    }

    fn top_down_msgs(&self, _height: BlockHeight, _nonce: u64) -> Vec<CrossMsg> {
        todo!()
    }

    fn membership(&self) -> Optioin<ValidatorSet> {
        todo!()
    }

    fn on_finality_committed(&self, finality: &IPCParentFinality) {
        let mut cache = self.cache.write().unwrap();
        cache.remove_key_till(finality.height);
    }
}

impl Clone for PollingParentSyncer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            started: Arc::new(AtomicBool::new(false)),
            cache: self.cache.clone(),
            ipc_agent_proxy: self.ipc_agent_proxy.clone(),
            handle: None
        }
    }
}

/// The cache for parent syncer
struct RangeKeyCache<Key, Value> {
    /// Stores the data in a hashmap.
    data: HashMap<Key, Value>,
    lower_bound: Key,
    upper_bound: Key
}

impl <Key: PrimInt + Hash, Value> RangeKeyCache<Key, Value> {
    pub fn new(mut key_pairs: Vec<(Key, Value)>) -> Self {
        if key_pairs.is_empty() {

        }

        let mut data = HashMap::new();
        let (mut lower, mut upper) = match key_pairs.pop() {
            None => {
                return Self {
                    data,
                    lower_bound: Key::zero(),
                    upper_bound: Key::zero()
                };
            }
            Some((key, val)) => {
                data.insert(key, val);
                (key, key)
            }
        };

        for (key, val) in key_pairs {
            if key < lower {
                lower = key;
            } else if key > upper {
                upper = key;
            }
            data.insert(key, val);
        }

        Self { data, lower_bound: lower, upper_bound: upper }
    }

    pub fn latest_key(&self) -> Key {
        self.upper_bound
    }

    pub fn get_value(&self, key: Key) -> Option<&Value> {
        if self.lower_bound > key || self.upper_bound < key {
            return None;
        }
        return self.data.get(&key);
    }

    /// Removes the block hashes stored till the specified height, exclusive.
    pub fn remove_key_till(&mut self, key: Key) {
        if self.lower_bound > key || self.upper_bound < key {
            return;
        }

        let mut i = self.lower_bound;
        while i < key {
            self.data.remove(&i);
            i = i + Key::one();
        }
    }

    /// Insert the block hash at the next height
    pub fn insert_after_lower_bound(&mut self, key: Key, val: Value) -> bool {
        if self.lower_bound > key {
            return false;
        }
        if self.upper_bound < key {
            self.upper_bound = key;
        }

        self.data.insert(key, val);
        true
    }
}

type LockedCache = Arc<RwLock<RangeKeyCache<BlockHeight, Vec<u8>>>>;

impl PollingParentSyncer {
    fn read_cache<F, T>(&self, f: F) -> T
    where
        F: Fn(&RangeKeyCache<BlockHeight, Vec<u8>>) -> T,
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
            cache.insert_after_lower_bound(r.0, r.1);
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
        let starting_height = cache.latest_key();
        tracing::debug!("polling parent from {starting_height:} till {latest_height:}");

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
