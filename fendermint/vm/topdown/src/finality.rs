// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::cache::{SequentialKeyCache};
use crate::error::Error;
use crate::{BlockHash, BlockHeight, Bytes, Config, IPCParentFinality, Nonce, ParentFinalityProvider, ParentViewProvider};
use async_stm::{abort, atomically, StmDynResult, StmResult, TVar};
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
        Ok(cache.values().cloned().collect())
    }
}

// TODO: keep it first, might be useful later
// macro_rules! downcast_err {
//     ($r:ident) => {
//         match $r {
//             Ok(()) => Ok(()),
//             Err(e) => match e.downcast_ref::<Error>() {
//                 None => unreachable!(),
//                 Some(e) => Err(e.clone())
//             }
//         }
//     }
// }

#[async_trait]
impl ParentViewProvider for DefaultFinalityProvider {
    async fn latest_height(&self) -> StmDynResult<Option<BlockHeight>> {
        let h = self.parent_view_data.latest_height()?;
        Ok(h)
    }

    async fn latest_nonce(&self) -> StmDynResult<Option<Nonce>> {
        let top_down_msgs = self.parent_view_data.top_down_msgs.read()?;
        Ok(top_down_msgs.upper_bound())
    }

    async fn new_block_height(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_set: ValidatorSet,
    ) -> StmDynResult<()> {
        let insert_res = self.parent_view_data.height_data.modify(|mut cache| {
            let r = cache.append(height, (block_hash.clone(), validator_set.clone()));
            (cache, r)
        })?;

        match insert_res {
            Ok(_) => Ok(()),
            Err(_) => {
                // now the inserted height is not the next expected block height, could be a chain
                // reorg if the caller is behaving correctly.
                abort(Error::ParentReorgDetected(height))
            }
        }
    }

    async fn new_top_down_msgs(&self, top_down_msgs: Vec<CrossMsg>) -> StmDynResult<()> {
        if top_down_msgs.is_empty() {
            // not processing if there are no top down msgs
            return Ok(());
        }

        // get the min nonce from the list of top down msgs and purge all the msgs with nonce
        // about the min nonce in cache, as the data should be newer and more accurate.
        let min_nonce = top_down_msgs.first().unwrap().msg.nonce;
        self.parent_view_data.top_down_msgs.update(|mut cache| {
            cache.remove_key_above(min_nonce);

            for msg in top_down_msgs.clone() {
                // safe to unwrap, as the append is sequential
                cache.append(msg.msg.nonce, msg).unwrap();
            }
            cache
        })?;

        Ok(())
    }
}

#[async_trait]
impl ParentFinalityProvider for DefaultFinalityProvider {
    async fn last_committed_finality(&self) -> StmDynResult<IPCParentFinality> {
        let finality = self.last_committed_finality.read_clone()?;
        Ok(finality)
    }

    async fn next_proposal(&self) -> StmDynResult<IPCParentFinality> {
        let latest_height = if let Some(h) = self.parent_view_data.latest_height()? {
            h
        } else {
            return abort(Error::HeightNotReady);
        };

        // latest height has not reached, we should wait or abort
        if latest_height < self.config.chain_head_delay {
            return abort(Error::HeightThresholdNotReached);
        }

        let height = latest_height - self.config.chain_head_delay;

        let height_data = self.parent_view_data.height_data.read()?;
        let (block_hash, validator_set) = if let Some(v) = height_data.get_value(height) {
            v.clone()
        } else {
            return abort(Error::HeightNotFoundInCache(height));
        };

        let top_down_msgs = self.parent_view_data.all_top_down_msgs()?;

        Ok(IPCParentFinality {
            height,
            block_hash,
            top_down_msgs,
            validator_set,
        })
    }

    async fn check_proposal(&self, proposal: &IPCParentFinality) -> StmDynResult<()> {
        self.check_height(proposal)?;
        self.check_block_hash(proposal)?;
        self.check_validator_set(proposal)?;
        self.check_top_down_msgs(proposal)
    }

    async fn on_finality_committed(&self, finality: &IPCParentFinality) -> StmDynResult<()> {
        // the nonce to clear
        let nonce = if !finality.top_down_msgs.is_empty() {
            let idx = finality.top_down_msgs.len() - 1;
            finality.top_down_msgs.get(idx).unwrap().msg.nonce
        } else {
            0
        };

        // the height to clear
        let height = finality.height;

        self.parent_view_data.height_data.update(|mut cache| {
            cache.remove_key_below(height + 1);
            cache
        })?;

        self.parent_view_data.top_down_msgs.update(|mut cache| {
            cache.remove_key_below(nonce + 1);
            cache
        })?;

        self.last_committed_finality.write(finality.clone())?;

        Ok(())
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

    fn check_height(&self, proposal: &IPCParentFinality) -> StmDynResult<()> {
        let latest_height = if let Some(h) = self.parent_view_data.latest_height()? {
            h
        } else {
            return abort(Error::HeightNotReady);
        };

        if latest_height < proposal.height {
            return abort(Error::ExceedingLatestHeight {
                proposal: proposal.height,
                parent: latest_height,
            });
        }

        let last_committed_finality = self.last_committed_finality.read()?;
        if proposal.height <= last_committed_finality.height {
            return abort(Error::HeightAlreadyCommitted(proposal.height));
        }

        Ok(())
    }

    fn check_block_hash(&self, proposal: &IPCParentFinality) -> StmDynResult<()> {
        if let Some(block_hash) = self.parent_view_data.block_hash(proposal.height)? {
            if block_hash == proposal.block_hash {
                return Ok(());
            }
            return abort(Error::BlockHashNotMatch {
                proposal: proposal.block_hash.clone(),
                parent: block_hash,
                height: proposal.height,
            });
        }
        abort(Error::BlockHashNotFound(proposal.height))
    }

    fn check_validator_set(&self, proposal: &IPCParentFinality) -> StmDynResult<()> {
        if let Some(validator_set) = self.parent_view_data.validator_set(proposal.height)? {
            if validator_set != proposal.validator_set {
                return abort(Error::ValidatorSetNotMatch(proposal.height));
            }
            return Ok(());
        }
        abort(Error::ValidatorSetNotFound(proposal.height))
    }

    fn check_top_down_msgs(&self, proposal: &IPCParentFinality) -> StmDynResult<()> {
        let last_committed_finality = self.last_committed_finality.read()?;
        if last_committed_finality.top_down_msgs.is_empty() || proposal.top_down_msgs.is_empty() {
            return Ok(());
        }

        let msg = last_committed_finality.top_down_msgs.last().unwrap();
        let max_nonce = msg.msg.nonce;
        let proposal_min_nonce = proposal.top_down_msgs.first().unwrap().msg.nonce;

        if max_nonce >= proposal_min_nonce {
            return abort(Error::InvalidNonce {
                proposal: proposal_min_nonce,
                parent: max_nonce,
                block: proposal.height,
            });
        }

        Ok(())
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
            .new_block_height(
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
                .new_block_height(
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
                .new_block_height(
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
                .new_block_height(
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
