// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::cache::SequentialKeyCache;
use crate::error::Error;
use crate::sync::IPCAgentProxy;
use crate::{
    BlockHash, BlockHeight, Config, IPCParentFinality, ParentFinalityProvider, ParentViewProvider,
};
use async_stm::{abort, atomically, Stm, StmResult, TVar};
use ipc_agent_sdk::message::ipc::ValidatorSet;
use ipc_sdk::cross::CrossMsg;
use std::sync::Arc;
use std::time::Duration;

type ParentViewPayload = (BlockHash, ValidatorSet, Vec<CrossMsg>);

/// The default parent finality provider
#[derive(Clone)]
pub struct CachedFinalityProvider {
    config: Config,
    /// Cached data that always syncs with the latest parent chain proactively
    cached_data: CachedData,
    /// This is a in memory view of the committed parent finality. We need this as a starting point
    /// for populating the cache
    last_committed_finality: TVar<Option<IPCParentFinality>>,
    /// The ipc agent proxy that works as a back up if cache miss
    agent: Arc<IPCAgentProxy>,
}

/// Tracks the data from the parent
#[derive(Clone)]
struct CachedData {
    height_data: TVar<SequentialKeyCache<BlockHeight, ParentViewPayload>>,
}

/// Exponential backoff for futures
macro_rules! retry {
    ($wait:expr, $retires:expr, $f:expr) => {{
        let mut retries = $retires;
        let mut wait = $wait;
        loop {
            match $f {
                Err(e) => {
                    tracing::warn!(
                        "cannot query ipc agent due to: {e}, retires: {retries}, wait: {wait}"
                    );
                    if retries > 0 {
                        retries -= 1;
                        tokio::time::sleep(Duration::from_secs(wait)).await;
                        wait *= 2;
                    }
                }
                res => break res,
            }
        }
    }};
}

#[async_trait::async_trait]
impl ParentViewProvider for CachedFinalityProvider {
    /// Should always return the validator set, only when ipc agent is down after exponeitial
    /// retries
    async fn validator_set(&self, height: BlockHeight) -> anyhow::Result<ValidatorSet> {
        let r = atomically(|| self.cached_data.validator_set(height)).await;
        if let Some(v) = r {
            return Ok(v);
        }

        retry!(
            self.config.exponential_back_off_secs,
            self.config.exponential_retry_limit,
            self.agent.get_validator_set(height).await
        )
    }

    /// Should always return the top down messages, only when ipc agent is down after exponeitial
    /// retries
    async fn top_down_msgs(&self, height: BlockHeight) -> anyhow::Result<Vec<CrossMsg>> {
        let r = atomically(|| self.cached_data.top_down_msgs_at_height(height)).await;
        if let Some(v) = r {
            return Ok(v);
        }

        retry!(
            self.config.exponential_back_off_secs,
            self.config.exponential_retry_limit,
            self.agent.get_top_down_msgs(height, height).await
        )
    }
}

impl ParentFinalityProvider for CachedFinalityProvider {
    fn next_proposal(&self) -> Stm<Option<IPCParentFinality>> {
        let height = if let Some(h) = self.cached_data.latest_height()? {
            h
        } else {
            return Ok(None);
        };

        // safe to unwrap as latest height exists
        let block_hash = self.cached_data.block_hash(height)?.unwrap();

        Ok(Some(IPCParentFinality { height, block_hash }))
    }

    fn check_proposal(&self, proposal: &IPCParentFinality) -> Stm<bool> {
        if !self.check_height(proposal)? {
            return Ok(false);
        }
        self.check_block_hash(proposal)
    }

    fn set_new_finality(&self, finality: IPCParentFinality) -> Stm<()> {
        // the height to clear
        let height = finality.height;

        self.cached_data.height_data.update(|mut cache| {
            cache.remove_key_below(height + 1);
            cache
        })?;

        self.last_committed_finality.write(Some(finality))
    }
}

impl CachedFinalityProvider {
    /// Creates an uninitialized provider
    /// We need this because `fendermint` has yet to be initialized and might
    /// not be able to provide an existing finality from the storage. This provider requires an
    /// existing committed finality. Providing the finality will enable other functionalities.
    pub fn uninitialized(config: Config, agent: Arc<IPCAgentProxy>) -> Self {
        Self::new(config, None, agent)
    }

    /// Creates an initialized provider
    pub fn initialized(
        config: Config,
        committed_finality: IPCParentFinality,
        agent: Arc<IPCAgentProxy>,
    ) -> Self {
        Self::new(config, Some(committed_finality), agent)
    }

    fn new(
        config: Config,
        committed_finality: Option<IPCParentFinality>,
        agent: Arc<IPCAgentProxy>,
    ) -> Self {
        let height_data = SequentialKeyCache::sequential();
        Self {
            config,
            cached_data: CachedData {
                height_data: TVar::new(height_data),
            },
            last_committed_finality: TVar::new(committed_finality),
            agent,
        }
    }

    pub fn latest_height(&self) -> Stm<Option<BlockHeight>> {
        self.cached_data.latest_height()
    }

    pub fn last_committed_finality(&self) -> Stm<Option<IPCParentFinality>> {
        self.last_committed_finality.read_clone()
    }

    pub fn new_parent_view(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_set: ValidatorSet,
        top_down_msgs: Vec<CrossMsg>,
    ) -> StmResult<(), Error> {
        if !top_down_msgs.is_empty() {
            // make sure incoming top down messages are ordered by nonce sequentially
            ensure_sequential_by_nonce(&top_down_msgs)?;
        };

        let r = self.cached_data.height_data.modify(|mut cache| {
            let r = cache
                .append(height, (block_hash, validator_set, top_down_msgs))
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

        let latest_height = if let Some(h) = self.cached_data.latest_height()? {
            h
        } else {
            // latest height is not found, meaning we dont have any prefetched cache, we just be
            // optimistic and vote yes as the previous latest committed finality check is passed
            // and it's valid.
            return Ok(true);
        };

        // requires the incoming height cannot be more advanced than our trusted parent node
        Ok(latest_height >= proposal.height)
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
    fn latest_height(&self) -> Stm<Option<BlockHeight>> {
        let cache = self.height_data.read()?;
        Ok(cache.upper_bound())
    }

    fn block_hash(&self, height: BlockHeight) -> Stm<Option<BlockHash>> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).map(|i| i.0.clone()))
    }

    fn validator_set(&self, height: BlockHeight) -> Stm<Option<ValidatorSet>> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).map(|i| i.1.clone()))
    }

    fn top_down_msgs_at_height(&self, height: BlockHeight) -> Stm<Option<Vec<CrossMsg>>> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).map(|i| i.2.clone()))
    }
}

fn ensure_sequential_by_nonce(msgs: &[CrossMsg]) -> StmResult<(), Error> {
    if msgs.is_empty() {
        return Ok(());
    }

    let mut nonce = msgs.first().unwrap().msg.nonce;
    for msg in msgs.iter().skip(1) {
        if nonce + 1 != msg.msg.nonce {
            return abort(Error::NonceNotSequential);
        }
        nonce += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::sync::IPCAgentProxy;
    use crate::{CachedFinalityProvider, Config, IPCParentFinality, ParentFinalityProvider};
    use async_stm::atomically_or_err;
    use fvm_shared::address::Address;
    use fvm_shared::econ::TokenAmount;
    use ipc_agent_sdk::apis::IpcAgentClient;
    use ipc_agent_sdk::jsonrpc::JsonRpcClientImpl;
    use ipc_agent_sdk::message::ipc::ValidatorSet;
    use ipc_sdk::cross::{CrossMsg, StorableMsg};
    use ipc_sdk::subnet_id::SubnetID;
    use std::str::FromStr;
    use std::sync::Arc;

    fn mocked_agent_proxy() -> Arc<IPCAgentProxy> {
        let unqueriable_agent = Arc::new(
            IPCAgentProxy::new(
                IpcAgentClient::new(JsonRpcClientImpl::new(
                    "http://localhost:3030/json_rpc".parse().unwrap(),
                    None,
                )),
                SubnetID::from_str("/r123/f410fgbphbzatgylhgw7u4w5idbc7pwka2upfienikky").unwrap(),
            )
            .unwrap(),
        );
        unqueriable_agent
    }

    fn new_provider() -> CachedFinalityProvider {
        let config = Config {
            chain_head_delay: 20,
            polling_interval_secs: 10,
            ipc_agent_url: "".to_string(),
            exponential_back_off_secs: 10,
            exponential_retry_limit: 10,
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        };

        CachedFinalityProvider::new(config, Some(genesis_finality), mocked_agent_proxy())
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

            provider.new_parent_view(
                10,
                vec![1u8; 32],
                ValidatorSet {
                    validators: None,
                    configuration_number: 0,
                },
                vec![],
            )?;

            let r = provider.next_proposal()?;
            assert!(r.is_some());

            // inject data
            for i in 11..=100 {
                provider.new_parent_view(
                    i,
                    vec![1u8; 32],
                    ValidatorSet {
                        validators: None,
                        configuration_number: i,
                    },
                    vec![],
                )?;
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

            assert_eq!(provider.latest_height()?, Some(100));

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
                provider.new_parent_view(
                    i,
                    vec![1u8; 32],
                    ValidatorSet {
                        validators: None,
                        configuration_number: 0,
                    },
                    vec![],
                )?;
            }

            let target_block = 120;
            let finality = IPCParentFinality {
                height: target_block,
                block_hash: vec![1u8; 32],
            };
            provider.set_new_finality(finality.clone())?;

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
            provider.new_parent_view(
                target_block,
                vec![1u8; 32],
                ValidatorSet {
                    validators: None,
                    configuration_number: 0,
                },
                vec![],
            )?;
            provider.set_new_finality(IPCParentFinality {
                height: target_block - 1,
                block_hash: vec![1u8; 32],
            })?;

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
            polling_interval_secs: 10,
            ipc_agent_url: "".to_string(),
            exponential_back_off_secs: 10,
            exponential_retry_limit: 10,
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        };

        let provider =
            CachedFinalityProvider::new(config, Some(genesis_finality), mocked_agent_proxy());

        let cross_msgs_batch1 = vec![new_cross_msg(0), new_cross_msg(1), new_cross_msg(2)];
        let cross_msgs_batch2 = vec![new_cross_msg(3), new_cross_msg(4), new_cross_msg(5)];
        let cross_msgs_batch3 = vec![new_cross_msg(6), new_cross_msg(7), new_cross_msg(8)];
        let cross_msgs_batch4 = vec![new_cross_msg(9), new_cross_msg(10), new_cross_msg(11)];

        atomically_or_err(|| {
            provider.new_parent_view(
                100,
                vec![1u8; 32],
                ValidatorSet {
                    validators: None,
                    configuration_number: 0,
                },
                cross_msgs_batch1.clone(),
            )?;

            provider.new_parent_view(
                101,
                vec![1u8; 32],
                ValidatorSet {
                    validators: None,
                    configuration_number: 0,
                },
                cross_msgs_batch2.clone(),
            )?;

            provider.new_parent_view(
                102,
                vec![1u8; 32],
                ValidatorSet {
                    validators: None,
                    configuration_number: 0,
                },
                cross_msgs_batch3.clone(),
            )?;
            provider.new_parent_view(
                103,
                vec![1u8; 32],
                ValidatorSet {
                    validators: None,
                    configuration_number: 0,
                },
                cross_msgs_batch4.clone(),
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
}
