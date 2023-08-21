// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::cache::{SequentialCacheInsert, SequentialKeyCache};
use crate::error::Error;
use crate::{
    BlockHeight, Bytes, Config, IPCParentFinality, Nonce, ParentFinalityProvider,
    ParentViewProvider,
};
use async_stm::{atomically, StmResult, TVar};
use async_trait::async_trait;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;

/// The default parent finality provider
pub struct DefaultFinalityProvider {
    config: Config,
    parent_view_data: ParentViewData,
    /// This is a in memory view of the committed parent finality,
    /// it should be synced with the store committed finality, owner of the struct should enforce
    /// this.
    last_committed_finality: TVar<IPCParentFinality>,
}

/// Tracks the data from the parent
#[derive(Clone)]
struct ParentViewData {
    height_data: TVar<SequentialKeyCache<BlockHeight, (Bytes, ValidatorSet)>>,
    top_down_msgs: TVar<SequentialKeyCache<Nonce, CrossMsg>>,
}

impl ParentViewData {
    fn latest_height(&self) -> StmResult<Option<BlockHeight>> {
        let cache = self.height_data.read()?;
        // safe to unwrap, we dont allow no upper bound
        Ok(cache.upper_bound())
    }

    fn block_hash(&self, height: BlockHeight) -> StmResult<Option<Bytes>> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).map(|i| i.0.clone()))
    }

    fn validator_set(&self, height: BlockHeight) -> StmResult<Option<ValidatorSet>> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).map(|i| i.1.clone()))
    }

    fn all_top_down_msgs(&self) -> StmResult<Vec<CrossMsg>> {
        let cache = self.top_down_msgs.read()?;
        Ok(cache.values().into_iter().cloned().collect())
    }
}

#[async_trait]
impl ParentViewProvider for DefaultFinalityProvider {
    async fn latest_height(&self) -> Option<BlockHeight> {
        atomically(|| self.parent_view_data.latest_height()).await
    }

    async fn latest_nonce(&self) -> Option<Nonce> {
        atomically(|| {
            let top_down_msgs = self.parent_view_data.top_down_msgs.read()?;
            Ok(top_down_msgs.upper_bound())
        })
        .await
    }

    async fn new_parent_view(
        &self,
        block_info: Option<(BlockHeight, Bytes, ValidatorSet)>,
        mut top_down_msgs: Vec<CrossMsg>,
    ) -> Result<(), Error> {
        top_down_msgs.sort_unstable_by(|a, b| {
            a.msg.nonce.cmp(&b.msg.nonce)
        });

        atomically(|| {
            if let Some((height, hash, validator_set)) = &block_info {
                let insert_res = self.parent_view_data.height_data.modify(|mut cache| {
                    let r = cache.insert(*height, (hash.clone(), validator_set.clone()));
                    (cache, r)
                })?;
                match insert_res {
                    SequentialCacheInsert::Ok => {}
                    // now the inserted height is not the next expected block height, could be a chain
                    // reorg if the caller is behaving correctly.
                    _ => return Ok(Err(Error::ParentReorgDetected(*height))),
                };
            }

            if top_down_msgs.is_empty() {
                // not processing if there are no top down msgs
                return Ok(Ok(()));
            }

            // get the min nonce from the list of top down msgs and purge all the msgs with nonce
            // about the min nonce in cache, as the data should be newer and more accurate.
            let min_nonce = top_down_msgs.first().unwrap().msg.nonce;
            self.parent_view_data.top_down_msgs.modify(|mut cache| {
                cache.remove_key_above(min_nonce);

                for msg in top_down_msgs.clone() {
                    cache.insert(msg.msg.nonce, msg);
                }
                (cache, ())
            })?;

            Ok(Ok(()))
        })
        .await
    }
}

#[async_trait]
impl ParentFinalityProvider for DefaultFinalityProvider {
    async fn last_committed_finality(&self) -> IPCParentFinality {
        atomically(|| {
            let finality = self.last_committed_finality.read_clone()?;
            Ok(finality)
        })
        .await
    }

    async fn next_proposal(&self) -> Result<IPCParentFinality, Error> {
        atomically(|| {
            let latest_height = if let Some(h) = self.parent_view_data.latest_height()? {
                h
            } else {
                return Ok(Err(Error::HeightNotReady));
            };

            // latest height has not reached, we should wait or abort
            if latest_height < self.config.chain_head_delay {
                return Ok(Err(Error::HeightThresholdNotReached));
            }

            let height = latest_height - self.config.chain_head_delay;

            let height_data = self.parent_view_data.height_data.read()?;
            let (block_hash, validator_set) = if let Some(v) = height_data.get_value(height) {
                v.clone()
            } else {
                return Ok(Err(Error::HeightNotFoundInCache(height)));
            };

            let top_down_msgs = self.parent_view_data.all_top_down_msgs()?;

            Ok(Ok(IPCParentFinality {
                height,
                block_hash,
                top_down_msgs,
                validator_set,
            }))
        })
        .await
    }

    async fn check_proposal(&self, proposal: &IPCParentFinality) -> Result<(), Error> {
        atomically(|| {
            let r = self.check_height(proposal)?;
            if r.is_err() {
                return Ok(r);
            }

            let r = self.check_block_hash(proposal)?;
            if r.is_err() {
                return Ok(r);
            }

            let r = self.check_validator_set(proposal)?;
            if r.is_err() {
                return Ok(r);
            }

            self.check_top_down_msgs(proposal)
        })
        .await
    }

    async fn on_finality_committed(&self, finality: &IPCParentFinality) {
        // the nonce to clear
        let nonce = if !finality.top_down_msgs.is_empty() {
            let idx = finality.top_down_msgs.len() - 1;
            finality.top_down_msgs.get(idx).unwrap().msg.nonce
        } else {
            0
        };

        // the height to clear
        let height = finality.height;

        atomically(|| {
            self.parent_view_data.height_data.modify(|mut cache| {
                cache.remove_key_below(height + 1);
                (cache, ())
            })?;

            self.parent_view_data.top_down_msgs.modify(|mut cache| {
                cache.remove_key_below(nonce + 1);
                (cache, ())
            })?;

            self.last_committed_finality.write(finality.clone())?;

            Ok(())
        })
        .await;
    }
}

impl DefaultFinalityProvider {
    pub fn new(config: Config, committed_finality: IPCParentFinality) -> Self {
        let height_data = SequentialKeyCache::new(config.block_interval);
        // nonce should be cached with increment 1
        let top_down_msgs = SequentialKeyCache::new(1);

        Self {
            config,
            parent_view_data: ParentViewData {
                height_data: TVar::new(height_data),
                top_down_msgs: TVar::new(top_down_msgs),
            },
            last_committed_finality: TVar::new(committed_finality),
        }
    }

    fn check_height(&self, proposal: &IPCParentFinality) -> StmResult<Result<(), Error>> {
        let latest_height = if let Some(h) = self.parent_view_data.latest_height()? {
            h
        } else {
            return Ok(Err(Error::HeightNotReady));
        };

        if latest_height < proposal.height {
            return Ok(Err(Error::ExceedingLatestHeight {
                proposal: proposal.height,
                parent: latest_height,
            }));
        }

        let last_committed_finality = self.last_committed_finality.read()?;
        if proposal.height <= last_committed_finality.height {
            return Ok(Err(Error::HeightAlreadyCommitted(proposal.height)));
        }

        Ok(Ok(()))
    }

    fn check_block_hash(&self, proposal: &IPCParentFinality) -> StmResult<Result<(), Error>> {
        if let Some(block_hash) = self.parent_view_data.block_hash(proposal.height)? {
            if block_hash == proposal.block_hash {
                return Ok(Ok(()));
            }
            return Ok(Err(Error::BlockHashNotMatch {
                proposal: proposal.block_hash.clone(),
                parent: block_hash,
                height: proposal.height,
            }));
        }
        Ok(Err(Error::BlockHashNotFound(proposal.height)))
    }

    fn check_validator_set(&self, proposal: &IPCParentFinality) -> StmResult<Result<(), Error>> {
        if let Some(validator_set) = self.parent_view_data.validator_set(proposal.height)? {
            if validator_set != proposal.validator_set {
                return Ok(Err(Error::ValidatorSetNotMatch(proposal.height)));
            }
            return Ok(Ok(()));
        }
        Ok(Err(Error::BlockHashNotFound(proposal.height)))
    }

    fn check_top_down_msgs(&self, proposal: &IPCParentFinality) -> StmResult<Result<(), Error>> {
        let last_committed_finality = self.last_committed_finality.read()?;
        if last_committed_finality.top_down_msgs.is_empty() || proposal.top_down_msgs.is_empty() {
            return Ok(Ok(()));
        }

        let msg = last_committed_finality.top_down_msgs.last().unwrap();
        let max_nonce = msg.msg.nonce;
        let proposal_min_nonce = proposal.top_down_msgs.first().unwrap().msg.nonce;

        if max_nonce >= proposal_min_nonce {
            return Ok(Err(Error::InvalidNonce {
                proposal: proposal_min_nonce,
                parent: max_nonce,
                block: proposal.height,
            }));
        }

        Ok(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::{
        Config, DefaultFinalityProvider, IPCParentFinality, ParentFinalityProvider,
        ParentViewProvider,
    };
    use ipc_sdk::ValidatorSet;

    fn new_provider() -> DefaultFinalityProvider {
        let config = Config {
            chain_head_delay: 20,
            chain_head_lower_bound: 100,
            block_interval: 1,
            polling_interval: 10,
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
            top_down_msgs: vec![],
            validator_set: Default::default(),
        };

        DefaultFinalityProvider::new(config, genesis_finality)
    }

    #[tokio::test]
    async fn test_next_proposal_works() {
        let provider = new_provider();

        let r = provider.next_proposal().await;
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), Error::HeightNotReady);

        provider
            .new_parent_view(
                Some((10, vec![1u8; 32], ValidatorSet::new(vec![], 10))),
                vec![],
            )
            .await
            .unwrap();
        let r = provider.next_proposal().await;
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), Error::HeightThresholdNotReached);

        // inject data
        for i in 11..=100 {
            provider
                .new_parent_view(
                    Some((i, vec![1u8; 32], ValidatorSet::new(vec![], i))),
                    vec![],
                )
                .await
                .unwrap();
        }

        let proposal = provider.next_proposal().await.unwrap();
        let target_block = 100 - 20; // deduct chain head delay
        assert_eq!(
            proposal,
            IPCParentFinality {
                height: target_block,
                block_hash: vec![1u8; 32],
                top_down_msgs: vec![],
                validator_set: ValidatorSet::new(vec![], target_block),
            }
        );

        assert_eq!(provider.latest_height().await, Some(100));
    }

    #[tokio::test]
    async fn test_finality_works() {
        let provider = new_provider();

        // inject data
        for i in 10..=100 {
            provider
                .new_parent_view(
                    Some((i, vec![1u8; 32], ValidatorSet::new(vec![], i))),
                    vec![],
                )
                .await
                .unwrap();
        }

        let target_block = 120;
        let finality = IPCParentFinality {
            height: target_block,
            block_hash: vec![1u8; 32],
            top_down_msgs: vec![],
            validator_set: ValidatorSet::new(vec![], target_block),
        };
        provider.on_finality_committed(&finality).await;

        // all cache should be cleared
        let r = provider.next_proposal().await;
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), Error::HeightNotReady);

        let f = provider.last_committed_finality().await;
        assert_eq!(f, finality);
    }

    #[tokio::test]
    async fn test_check_proposal_works() {
        let provider = new_provider();

        // inject data
        for i in 20..=100 {
            provider
                .new_parent_view(
                    Some((i, vec![1u8; 32], ValidatorSet::new(vec![], i))),
                    vec![],
                )
                .await
                .unwrap();
        }

        let target_block = 120;
        let finality = IPCParentFinality {
            height: target_block,
            block_hash: vec![1u8; 32],
            top_down_msgs: vec![],
            validator_set: ValidatorSet::new(vec![], target_block),
        };

        let r = provider.check_proposal(&finality).await;
        assert!(r.is_err());
        assert_eq!(
            r.unwrap_err(),
            Error::ExceedingLatestHeight {
                proposal: 120,
                parent: 100
            }
        );

        let target_block = 100;
        let finality = IPCParentFinality {
            height: target_block,
            block_hash: vec![1u8; 32],
            top_down_msgs: vec![],
            validator_set: ValidatorSet::new(vec![], target_block),
        };

        assert!(provider.check_proposal(&finality).await.is_ok());
    }
}
