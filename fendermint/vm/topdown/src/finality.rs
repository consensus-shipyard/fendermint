// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::cache::SequentialKeyCache;
use crate::error::Error;
use crate::proxy::ParentQueryProxy;
use crate::{
    BlockHash, BlockHeight, Config, IPCParentFinality, ParentFinalityProvider, ParentViewProvider,
};
use async_stm::{abort, atomically, Stm, StmResult, TVar};
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::staking::StakingChangeRequest;
use std::sync::Arc;

pub(crate) type ParentViewPayload = (BlockHash, Vec<StakingChangeRequest>, Vec<CrossMsg>);

/// The default parent finality provider
#[derive(Clone)]
pub struct CachedFinalityProvider<T> {
    config: Config,
    genesis_epoch: BlockHeight,
    /// Cached data that always syncs with the latest parent chain proactively
    cached_data: CachedData,
    /// This is a in memory view of the committed parent finality. We need this as a starting point
    /// for populating the cache
    last_committed_finality: TVar<Option<IPCParentFinality>>,
    /// The ipc client proxy that works as a back up if cache miss
    parent_client: Arc<T>,
}

/// Tracks the data from the parent
#[derive(Clone)]
struct CachedData {
    height_data: TVar<SequentialKeyCache<BlockHeight, Option<ParentViewPayload>>>,
}

/// Exponential backoff for futures
macro_rules! retry {
    ($wait:expr, $retires:expr, $f:expr) => {{
        let mut retries = $retires;
        let mut wait = $wait;

        loop {
            let res = $f;
            if let Err(e) = &res {
                tracing::warn!(
                    "cannot query ipc parent_client due to: {e}, retires: {retries}, wait: {wait:?}"
                );
                if retries > 0 {
                    retries -= 1;

                    tokio::time::sleep(wait).await;

                    wait *= 2;
                    continue;
                }
            }

            break res;
        }
    }};
}

#[async_trait::async_trait]
impl<T: ParentQueryProxy + Send + Sync + 'static> ParentViewProvider for CachedFinalityProvider<T> {
    fn genesis_epoch(&self) -> anyhow::Result<BlockHeight> {
        Ok(self.genesis_epoch)
    }

    async fn validator_changes_from(
        &self,
        from: BlockHeight,
        to: BlockHeight,
    ) -> anyhow::Result<Vec<StakingChangeRequest>> {
        let mut v = vec![];
        for h in from..=to {
            let mut r = self.validator_changes(h).await?;
            tracing::debug!(
                "obtained validator change set (len {}) at height {h}",
                r.len()
            );
            v.append(&mut r);
        }

        Ok(v)
    }

    /// Should always return the validator set, only when ipc parent_client is down after exponeitial
    /// retries
    async fn validator_changes(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<Vec<StakingChangeRequest>> {
        let r = atomically(|| {
            let changes = self.cached_data.validator_changes(height)?;
            Ok(if changes.is_some() {
                changes
            } else if self.cached_data.contains(height)? {
                // we have a lotus null block
                Some(vec![])
            } else {
                None
            })
        })
        .await;

        if let Some(v) = r {
            return Ok(v);
        }

        retry!(
            self.config.exponential_back_off,
            self.config.exponential_retry_limit,
            self.parent_client
                .get_validator_changes(height)
                .await
                .map(|r| r.value)
        )
    }

    /// Should always return the top down messages, only when ipc parent_client is down after exponential
    /// retries
    async fn top_down_msgs(
        &self,
        height: BlockHeight,
        block_hash: &BlockHash,
    ) -> anyhow::Result<Vec<CrossMsg>> {
        let r = atomically(|| {
            let changes = self.cached_data.top_down_msgs_at_height(height)?;
            Ok(if changes.is_some() {
                changes
            } else if self.cached_data.contains(height)? {
                // we have a lotus null block
                Some(vec![])
            } else {
                None
            })
        })
        .await;

        if let Some(v) = r {
            return Ok(v);
        }

        retry!(
            self.config.exponential_back_off,
            self.config.exponential_retry_limit,
            self.parent_client
                .get_top_down_msgs_with_hash(height, block_hash)
                .await
        )
    }

    async fn top_down_msgs_from(
        &self,
        from: BlockHeight,
        to: BlockHeight,
        block_hash: &BlockHash,
    ) -> anyhow::Result<Vec<CrossMsg>> {
        let mut v = vec![];
        for h in from..=to {
            let mut r = self.top_down_msgs(h, block_hash).await?;
            v.append(&mut r);
        }
        Ok(v)
    }
}

impl<T: ParentQueryProxy + Send + Sync + 'static> ParentFinalityProvider
    for CachedFinalityProvider<T>
{
    fn next_proposal(&self) -> Stm<Option<IPCParentFinality>> {
        let height = if let Some(h) = self.cached_data.latest_height()? {
            h
        } else {
            tracing::debug!("no proposal yet as height not available");
            return Ok(None);
        };

        // safe to unwrap as latest height exists
        let block_hash = if let Some(h) = self.cached_data.block_hash(height)? {
            h
        } else {
            // Oops, we have a null round in parent, skip this proposal and wait for future blocks.
            return Ok(None);
        };

        let proposal = IPCParentFinality { height, block_hash };
        tracing::debug!("new proposal: {proposal:?}");
        Ok(Some(proposal))
    }

    fn check_proposal(&self, proposal: &IPCParentFinality) -> Stm<bool> {
        if !self.check_height(proposal)? {
            return Ok(false);
        }
        self.check_block_hash(proposal)
    }

    fn set_new_finality(
        &self,
        finality: IPCParentFinality,
        previous_finality: Option<IPCParentFinality>,
    ) -> Stm<()> {
        debug_assert!(previous_finality == self.last_committed_finality.read_clone()?);

        // the height to clear
        let height = finality.height;

        self.cached_data.height_data.update(|mut cache| {
            cache.remove_key_below(height + 1);
            cache
        })?;

        self.last_committed_finality.write(Some(finality))
    }
}

impl<T: ParentQueryProxy + Send + Sync + 'static> CachedFinalityProvider<T> {
    /// Creates an uninitialized provider
    /// We need this because `fendermint` has yet to be initialized and might
    /// not be able to provide an existing finality from the storage. This provider requires an
    /// existing committed finality. Providing the finality will enable other functionalities.
    pub async fn uninitialized(config: Config, parent_client: Arc<T>) -> anyhow::Result<Self> {
        let genesis = parent_client.get_genesis_epoch().await?;
        Ok(Self::new(config, genesis, None, parent_client))
    }
}

impl<T> CachedFinalityProvider<T> {
    fn new(
        config: Config,
        genesis_epoch: BlockHeight,
        committed_finality: Option<IPCParentFinality>,
        parent_client: Arc<T>,
    ) -> Self {
        let height_data = SequentialKeyCache::sequential();
        Self {
            config,
            genesis_epoch,
            cached_data: CachedData {
                height_data: TVar::new(height_data),
            },
            last_committed_finality: TVar::new(committed_finality),
            parent_client,
        }
    }

    pub fn block_hash(&self, height: BlockHeight) -> Stm<Option<BlockHash>> {
        self.cached_data.block_hash(height)
    }

    pub fn first_non_null_parent_hash(&self, height: BlockHeight) -> Stm<Option<BlockHash>> {
        let cache = self.cached_data.height_data.read()?;
        if let Some(lower_bound) = cache.lower_bound() {
            for h in (lower_bound..height).rev() {
                if let Some(Some(p)) = cache.get_value(h) {
                    return Ok(Some(p.0.clone()));
                }
            }
        }

        // nothing is found in cache, check the last committed finality
        let last_committed_finality = self.last_committed_finality.read_clone()?;
        if let Some(f) = last_committed_finality {
            Ok(Some(f.block_hash))
        } else {
            Ok(None)
        }
    }

    pub fn latest_height(&self) -> Stm<Option<BlockHeight>> {
        self.cached_data.latest_height()
    }

    pub fn last_committed_finality(&self) -> Stm<Option<IPCParentFinality>> {
        self.last_committed_finality.read_clone()
    }

    /// Clear the cache and set the committed finality to the provided value
    pub fn reset(&self, finality: IPCParentFinality) -> Stm<()> {
        self.cached_data
            .height_data
            .write(SequentialKeyCache::sequential())?;
        self.last_committed_finality.write(Some(finality))
    }

    pub fn new_parent_view(
        &self,
        height: BlockHeight,
        maybe_payload: Option<ParentViewPayload>,
    ) -> StmResult<(), Error> {
        if let Some((block_hash, validator_changes, top_down_msgs)) = maybe_payload {
            self.parent_block_filled(height, block_hash, validator_changes, top_down_msgs)
        } else {
            self.parent_null_round(height)
        }
    }

    fn parent_block_filled(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_changes: Vec<StakingChangeRequest>,
        top_down_msgs: Vec<CrossMsg>,
    ) -> StmResult<(), Error> {
        if !top_down_msgs.is_empty() {
            // make sure incoming top down messages are ordered by nonce sequentially
            tracing::debug!("top down messages: {top_down_msgs:#?}");
            ensure_sequential(&top_down_msgs, |msg| msg.msg.nonce)?;
        };
        if !validator_changes.is_empty() {
            tracing::debug!("validator changes: {validator_changes:#?}");
            ensure_sequential(&validator_changes, |change| change.configuration_number)?;
        }

        let r = self.cached_data.height_data.modify(|mut cache| {
            let r = cache
                .append(height, Some((block_hash, validator_changes, top_down_msgs)))
                .map_err(Error::NonSequentialParentViewInsert);
            (cache, r)
        })?;

        if let Err(e) = r {
            return abort(e);
        }

        Ok(())
    }

    /// When there is a new parent view, but it is actually a null round, call this function.
    fn parent_null_round(&self, height: BlockHeight) -> StmResult<(), Error> {
        let r = self.cached_data.height_data.modify(|mut cache| {
            let r = cache
                .append(height, None)
                .map_err(Error::NonSequentialParentViewInsert);
            (cache, r)
        })?;

        if let Err(e) = r {
            return abort(e);
        }

        Ok(())
    }

    fn check_height(&self, proposal: &IPCParentFinality) -> Stm<bool> {
        let binding = self.last_committed_finality.read()?;
        // last committed finality is not ready yet, we don't vote, just reject
        let last_committed_finality = if let Some(f) = binding.as_ref() {
            f
        } else {
            return Ok(false);
        };

        // the incoming proposal has height already committed, reject
        if last_committed_finality.height >= proposal.height {
            return Ok(false);
        }

        if let Some(latest_height) = self.cached_data.latest_height()? {
            // requires the incoming height cannot be more advanced than our trusted parent node
            Ok(latest_height >= proposal.height)
        } else {
            // latest height is not found, meaning we dont have any prefetched cache, we just be
            // strict and vote no simply because we don't know..
            Ok(false)
        }
    }

    fn check_block_hash(&self, proposal: &IPCParentFinality) -> Stm<bool> {
        Ok(
            if let Some(block_hash) = self.cached_data.block_hash(proposal.height)? {
                block_hash == proposal.block_hash
            } else {
                false
            },
        )
    }
}

impl CachedData {
    /// Checks if the cache contain the block height's data
    fn contains(&self, height: BlockHeight) -> Stm<bool> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).is_none())
    }

    fn latest_height(&self) -> Stm<Option<BlockHeight>> {
        let cache = self.height_data.read()?;
        Ok(cache.upper_bound())
    }

    fn block_hash(&self, height: BlockHeight) -> Stm<Option<BlockHash>> {
        self.get_at_height(height, |i| i.0.clone())
    }

    fn validator_changes(&self, height: BlockHeight) -> Stm<Option<Vec<StakingChangeRequest>>> {
        self.get_at_height(height, |i| i.1.clone())
    }

    fn top_down_msgs_at_height(&self, height: BlockHeight) -> Stm<Option<Vec<CrossMsg>>> {
        self.get_at_height(height, |i| i.2.clone())
    }

    fn get_at_height<T, F: Fn(&ParentViewPayload) -> T>(
        &self,
        height: BlockHeight,
        f: F,
    ) -> Stm<Option<T>> {
        let cache = self.height_data.read()?;
        Ok(if let Some(Some(v)) = cache.get_value(height) {
            Some(f(v))
        } else {
            None
        })
    }
}

fn ensure_sequential<T, F: Fn(&T) -> u64>(msgs: &[T], f: F) -> StmResult<(), Error> {
    if msgs.is_empty() {
        return Ok(());
    }

    let first = msgs.first().unwrap();
    let mut nonce = f(first);
    for msg in msgs.iter().skip(1) {
        if nonce + 1 != f(msg) {
            return abort(Error::NotSequential);
        }
        nonce += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::proxy::ParentQueryProxy;
    use crate::{
        BlockHash, BlockHeight, CachedFinalityProvider, Config, IPCParentFinality,
        ParentFinalityProvider,
    };
    use async_stm::atomically_or_err;
    use async_trait::async_trait;
    use fvm_shared::address::Address;
    use fvm_shared::econ::TokenAmount;
    use ipc_provider::manager::{GetBlockHashResult, TopDownQueryPayload};
    use ipc_sdk::cross::{CrossMsg, StorableMsg};
    use ipc_sdk::staking::StakingChangeRequest;
    use ipc_sdk::subnet_id::SubnetID;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::time::Duration;

    struct MockedParentQuery;

    #[async_trait]
    impl ParentQueryProxy for MockedParentQuery {
        async fn get_chain_head_height(&self) -> anyhow::Result<BlockHeight> {
            Ok(1)
        }

        async fn get_genesis_epoch(&self) -> anyhow::Result<BlockHeight> {
            Ok(10)
        }

        async fn get_block_hash(&self, _height: BlockHeight) -> anyhow::Result<GetBlockHashResult> {
            Ok(GetBlockHashResult::default())
        }

        async fn get_top_down_msgs_with_hash(
            &self,
            _height: BlockHeight,
            _block_hash: &BlockHash,
        ) -> anyhow::Result<Vec<CrossMsg>> {
            Ok(vec![])
        }

        async fn get_validator_changes(
            &self,
            _height: BlockHeight,
        ) -> anyhow::Result<TopDownQueryPayload<Vec<StakingChangeRequest>>> {
            Ok(TopDownQueryPayload {
                value: vec![],
                block_hash: vec![],
            })
        }
    }

    fn mocked_agent_proxy() -> Arc<MockedParentQuery> {
        Arc::new(MockedParentQuery)
    }

    fn genesis_finality() -> IPCParentFinality {
        IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        }
    }

    fn new_provider() -> CachedFinalityProvider<MockedParentQuery> {
        let config = Config {
            chain_head_delay: 20,
            polling_interval: Duration::from_secs(10),
            exponential_back_off: Duration::from_secs(10),
            exponential_retry_limit: 10,
        };

        CachedFinalityProvider::new(config, 10, Some(genesis_finality()), mocked_agent_proxy())
    }

    fn new_cross_msg(nonce: u64) -> CrossMsg {
        let subnet_id = SubnetID::new(10, vec![Address::new_id(1000)]);
        let mut msg = StorableMsg::new_fund_msg(
            &subnet_id,
            &Address::new_id(1),
            &Address::new_id(2),
            TokenAmount::from_atto(100),
        )
        .unwrap();
        msg.nonce = nonce;

        CrossMsg {
            msg,
            wrapped: false,
        }
    }

    #[tokio::test]
    async fn test_next_proposal_works() {
        let provider = new_provider();

        atomically_or_err(|| {
            let r = provider.next_proposal()?;
            assert!(r.is_none());

            provider.new_parent_view(10, Some((vec![1u8; 32], vec![], vec![])))?;

            let r = provider.next_proposal()?;
            assert!(r.is_some());

            // inject data
            for i in 11..=100 {
                provider.new_parent_view(i, Some((vec![1u8; 32], vec![], vec![])))?;
            }

            let proposal = provider.next_proposal()?.unwrap();
            let target_block = 100;
            assert_eq!(
                proposal,
                IPCParentFinality {
                    height: target_block,
                    block_hash: vec![1u8; 32],
                }
            );

            assert_eq!(provider.latest_height()?.unwrap(), 100);

            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_finality_works() {
        let provider = new_provider();

        atomically_or_err(|| {
            // inject data
            for i in 10..=100 {
                provider.new_parent_view(i, Some((vec![1u8; 32], vec![], vec![])))?;
            }

            let target_block = 120;
            let finality = IPCParentFinality {
                height: target_block,
                block_hash: vec![1u8; 32],
            };
            provider.set_new_finality(finality.clone(), Some(genesis_finality()))?;

            // all cache should be cleared
            let r = provider.next_proposal()?;
            assert!(r.is_none());

            let f = provider.last_committed_finality()?;
            assert_eq!(f, Some(finality));

            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_check_proposal_works() {
        let provider = new_provider();

        atomically_or_err(|| {
            let target_block = 100;

            // inject data
            provider.new_parent_view(target_block, Some((vec![1u8; 32], vec![], vec![])))?;
            provider.set_new_finality(
                IPCParentFinality {
                    height: target_block - 1,
                    block_hash: vec![1u8; 32],
                },
                Some(genesis_finality()),
            )?;

            let finality = IPCParentFinality {
                height: target_block,
                block_hash: vec![1u8; 32],
            };

            assert!(provider.check_proposal(&finality).is_ok());

            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_top_down_msgs_works() {
        let config = Config {
            chain_head_delay: 2,
            polling_interval: Duration::from_secs(10),
            exponential_back_off: Duration::from_secs(10),
            exponential_retry_limit: 10,
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        };

        let provider =
            CachedFinalityProvider::new(config, 10, Some(genesis_finality), mocked_agent_proxy());

        let cross_msgs_batch1 = vec![new_cross_msg(0), new_cross_msg(1), new_cross_msg(2)];
        let cross_msgs_batch2 = vec![new_cross_msg(3), new_cross_msg(4), new_cross_msg(5)];
        let cross_msgs_batch3 = vec![new_cross_msg(6), new_cross_msg(7), new_cross_msg(8)];
        let cross_msgs_batch4 = vec![new_cross_msg(9), new_cross_msg(10), new_cross_msg(11)];

        atomically_or_err(|| {
            provider.new_parent_view(
                100,
                Some((vec![1u8; 32], vec![], cross_msgs_batch1.clone())),
            )?;

            provider.new_parent_view(
                101,
                Some((vec![1u8; 32], vec![], cross_msgs_batch2.clone())),
            )?;

            provider.new_parent_view(
                102,
                Some((vec![1u8; 32], vec![], cross_msgs_batch3.clone())),
            )?;
            provider.new_parent_view(
                103,
                Some((vec![1u8; 32], vec![], cross_msgs_batch4.clone())),
            )?;

            let mut v1 = cross_msgs_batch1.clone();
            let v2 = cross_msgs_batch2.clone();
            v1.extend(v2);
            let finality = IPCParentFinality {
                height: 103,
                block_hash: vec![1u8; 32],
            };
            let next_proposal = provider.next_proposal()?.unwrap();
            assert_eq!(next_proposal, finality);

            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_retry() {
        struct Test {
            nums_run: AtomicUsize,
        }

        impl Test {
            async fn run(&self) -> Result<(), &'static str> {
                self.nums_run.fetch_add(1, Ordering::SeqCst);
                Err("mocked error")
            }
        }

        let t = Test {
            nums_run: AtomicUsize::new(0),
        };

        let res = retry!(Duration::from_secs(1), 2, t.run().await);
        assert!(res.is_err());
        // execute the first time, retries twice
        assert_eq!(t.nums_run.load(Ordering::SeqCst), 3);
    }
}
