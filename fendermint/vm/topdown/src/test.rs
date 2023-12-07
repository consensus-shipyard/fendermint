// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::finality::ParentViewPayload;
use crate::proxy::ParentQueryProxy;
use crate::sync::{LotusParentSyncer, ParentFinalityStateQuery};
use crate::{
    BlockHeight, CachedFinalityProvider, Config, IPCParentFinality, ParentFinalityProvider,
    ParentViewProvider, SequentialKeyCache, Toggle, NULL_ROUND_ERR_MSG,
};
use anyhow::anyhow;
use async_stm::atomically;
use async_trait::async_trait;
use ipc_provider::manager::{GetBlockHashResult, TopDownQueryPayload};
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::staking::StakingChangeRequest;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

struct TestParentFinalityStateQuery {
    latest_finality: IPCParentFinality,
}

impl ParentFinalityStateQuery for TestParentFinalityStateQuery {
    fn get_latest_committed_finality(&self) -> anyhow::Result<Option<IPCParentFinality>> {
        Ok(Some(self.latest_finality.clone()))
    }
}

/// Creates a mock of a new parent blockchain view. The key is the height and the value is the
/// block hash. If block hash is None, it means the current height is a null block.
macro_rules! new_parent_blocks {
        ($($key:expr => $val:expr),* ,) => (
            hash_map!($($key => $val),*)
        );
        ($($key:expr => $val:expr),*) => ({
            let mut map = SequentialKeyCache::sequential();
            $( map.append($key, $val).unwrap(); )*
            map
        });
    }

struct TestParentProxy {
    blocks: SequentialKeyCache<BlockHeight, Option<ParentViewPayload>>,
    should_wait: Arc<AtomicBool>,
    tx: tokio::sync::mpsc::Sender<u8>,
}

#[async_trait]
impl ParentQueryProxy for TestParentProxy {
    async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
        Ok(self.blocks.upper_bound().unwrap())
    }

    async fn get_genesis_epoch(&self) -> anyhow::Result<BlockHeight> {
        Ok(self.blocks.lower_bound().unwrap() - 1)
    }

    async fn get_block_hash(&self, height: BlockHeight) -> anyhow::Result<GetBlockHashResult> {
        let r = self.blocks.get_value(height).unwrap();
        if r.is_none() {
            return Err(anyhow!(NULL_ROUND_ERR_MSG));
        }

        for h in (self.blocks.lower_bound().unwrap()..height).rev() {
            let v = self.blocks.get_value(h).unwrap();
            if v.is_none() {
                continue;
            }
            return Ok(GetBlockHashResult {
                parent_block_hash: v.clone().unwrap().0,
                block_hash: r.clone().unwrap().0,
            });
        }
        panic!("invalid testing data")
    }

    async fn get_top_down_msgs(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<TopDownQueryPayload<Vec<CrossMsg>>> {
        while self.should_wait.load(Ordering::SeqCst) {
            self.tx.send(0).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let r = self.blocks.get_value(height).cloned().unwrap();
        if r.is_none() {
            return Err(anyhow!(NULL_ROUND_ERR_MSG));
        }
        let r = r.unwrap();
        Ok(TopDownQueryPayload {
            value: r.2,
            block_hash: r.0,
        })
    }

    async fn get_validator_changes(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<TopDownQueryPayload<Vec<StakingChangeRequest>>> {
        let r = self.blocks.get_value(height).cloned().unwrap();
        if r.is_none() {
            return Err(anyhow!(NULL_ROUND_ERR_MSG));
        }
        let r = r.unwrap();
        Ok(TopDownQueryPayload {
            value: r.1,
            block_hash: r.0,
        })
    }
}

async fn new_setup(
    blocks: SequentialKeyCache<BlockHeight, Option<ParentViewPayload>>,
    should_wait: Arc<AtomicBool>,
    tx: tokio::sync::mpsc::Sender<u8>,
) -> (
    Arc<Toggle<CachedFinalityProvider<TestParentProxy>>>,
    LotusParentSyncer<TestParentFinalityStateQuery, TestParentProxy>,
) {
    let config = Config {
        chain_head_delay: 2,
        polling_interval: Default::default(),
        exponential_back_off: Default::default(),
        exponential_retry_limit: 0,
        max_proposal_range: Some(10),
        max_cache_blocks: None,
        proposal_delay: None,
    };
    let genesis_epoch = blocks.lower_bound().unwrap();
    let proxy = Arc::new(TestParentProxy {
        blocks,
        should_wait,
        tx,
    });
    let committed_finality = IPCParentFinality {
        height: genesis_epoch,
        block_hash: vec![0; 32],
    };

    let provider = CachedFinalityProvider::new(
        config.clone(),
        genesis_epoch,
        Some(committed_finality.clone()),
        proxy.clone(),
    );
    let provider = Arc::new(Toggle::enabled(provider));

    let syncer = LotusParentSyncer::new(
        config,
        proxy,
        provider.clone(),
        Arc::new(TestParentFinalityStateQuery {
            latest_finality: committed_finality,
        }),
    );

    (provider, syncer)
}

/// This test case is for when pulling the next block, there is a new commit finality request
/// and all cache is purged.
#[tokio::test]
async fn while_syncing_cache_purged() {
    let parent_blocks = new_parent_blocks!(
        100 => Some((vec![0; 32], vec![], vec![])),   // genesis block
        101 => Some((vec![1; 32], vec![], vec![])),
        102 => Some((vec![2; 32], vec![], vec![])),
        103 => Some((vec![3; 32], vec![], vec![])),
        104 => Some((vec![4; 32], vec![], vec![])),
        105 => Some((vec![5; 32], vec![], vec![])),
        106 => Some((vec![6; 32], vec![], vec![])),
        107 => Some((vec![6; 32], vec![], vec![])),
        108 => Some((vec![6; 32], vec![], vec![])),
        109 => Some((vec![6; 32], vec![], vec![])),
        110 => Some((vec![6; 32], vec![], vec![]))
    );
    let should_wait = Arc::new(AtomicBool::new(false));
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);

    let (provider, mut syncer) = new_setup(parent_blocks, should_wait.clone(), tx).await;
    syncer.sync().await.unwrap();
    syncer.sync().await.unwrap();
    syncer.sync().await.unwrap();
    assert_eq!(
        atomically(|| provider.next_proposal()).await,
        Some(IPCParentFinality {
            height: 101,
            block_hash: vec![1; 32]
        }),
        "sanity check, make sure there is data in cache"
    );

    let syncer = Arc::new(Mutex::new(syncer));
    // now make sure we are "waiting" for polling top down messages, like in real io
    should_wait.store(true, Ordering::SeqCst);
    let cloned_syncer = syncer.clone();
    let handle = tokio::spawn(async move {
        let mut syncer = cloned_syncer.lock().await;
        syncer.sync().await.unwrap();
    });

    loop {
        if let Some(_) = rx.recv().await {
            // syncer.sync is waiting, we mock a new proposal from peer, which the parent blockchain
            // has not seen before, i.e. 107
            atomically(|| {
                provider.set_new_finality(
                    IPCParentFinality {
                        height: 105,
                        block_hash: vec![5; 32],
                    },
                    Some(IPCParentFinality {
                        height: 100,
                        block_hash: vec![0; 32],
                    }),
                )
            })
            .await;
            should_wait.store(false, Ordering::SeqCst);
            break;
        }
    }
    handle.await.unwrap();
    assert_eq!(
        atomically(|| provider.next_proposal()).await,
        None,
        "cache should be evicted, so proposal should be made"
    );

    atomically(|| {
        assert_eq!(Some(105), provider.latest_height()?);
        assert_eq!(
            Some(IPCParentFinality {
                height: 105,
                block_hash: vec![5; 32]
            }),
            provider.last_committed_finality()?
        );
        Ok(())
    })
    .await;

    assert_eq!(
        provider
            .validator_changes_from(105, 105)
            .await
            .unwrap()
            .len(),
        0
    );

    // make sure syncer still works
    let mut syncer = syncer.lock().await;
    syncer.sync().await.unwrap();

    atomically(|| {
        assert_eq!(Some(106), provider.latest_height()?);
        assert_eq!(
            Some(IPCParentFinality {
                height: 105,
                block_hash: vec![5; 32]
            }),
            provider.last_committed_finality()?
        );
        Ok(())
    })
    .await;

    syncer.sync().await.unwrap();
    syncer.sync().await.unwrap();
    assert_eq!(
        atomically(|| provider.next_proposal()).await,
        Some(IPCParentFinality {
            height: 106,
            block_hash: vec![6; 32]
        }),
    );
}
