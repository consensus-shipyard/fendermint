// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::cache::SequentialKeyCache;
use crate::error::Error;
use crate::{
    BlockHash, BlockHeight, Bytes, Config, IPCParentFinality, Nonce, ParentFinalityProvider,
    ParentViewProvider,
};
use async_stm::{abort, StmDynResult, StmResult, TVar};
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;

type ParentViewPayload = (Bytes, ValidatorSet, Vec<CrossMsg>);

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
    height_data: TVar<SequentialKeyCache<BlockHeight, ParentViewPayload>>,
    /// Tracks the next top down nonce in the parent view, not the committed top down nonce. This value
    /// should be larger than the next nonce in last committed top down nonce.
    ///
    /// In `height_data`, cross messages are actually stored
    /// using block height. However, some block height might not have any cross messages. This means
    /// getting the latest top down nonce will traverse the height data. Tracking it separately will
    /// avoid the traversal.
    next_top_down_nonce: TVar<Nonce>,
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

    fn top_down_msgs(
        &self,
        from_height: BlockHeight,
        to_height: BlockHeight,
    ) -> StmResult<Vec<CrossMsg>> {
        let cache = self.height_data.read()?;
        let v = cache
            .values_within(from_height, to_height)
            .flat_map(|i| i.2.iter())
            .cloned()
            .collect();
        Ok(v)
    }

    fn next_nonce(&self) -> StmResult<Nonce> {
        let nonce = self.next_top_down_nonce.read()?;
        Ok(*nonce)
    }
}

impl ParentViewProvider for DefaultFinalityProvider {
    fn latest_height(&self) -> StmDynResult<Option<BlockHeight>> {
        let h = self.parent_view_data.latest_height()?;
        Ok(h)
    }

    fn next_nonce(&self) -> StmDynResult<Nonce> {
        Ok(self.parent_view_data.next_nonce()?)
    }

    fn new_parent_view(
        &self,
        height: BlockHeight,
        block_hash: BlockHash,
        validator_set: ValidatorSet,
        top_down_msgs: Vec<CrossMsg>,
    ) -> StmDynResult<()> {
        if !top_down_msgs.is_empty() {
            // make sure incoming top down messages are ordered by nonce sequentially
            ensure_sequential_by_nonce(&top_down_msgs)?;

            // make sure nonce is sequential
            let next_nonce = self.next_nonce()?;
            let min_incoming_nonce = top_down_msgs.first().unwrap().msg.nonce;
            if next_nonce != min_incoming_nonce {
                return abort(Error::NonceNotSequential);
            }

            let max_incoming_nonce = top_down_msgs.last().unwrap().msg.nonce;
            self.parent_view_data
                .next_top_down_nonce
                .write(max_incoming_nonce + 1)?;
        };

        let r = self.parent_view_data.height_data.modify(|mut cache| {
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
}

impl ParentFinalityProvider for DefaultFinalityProvider {
    fn last_committed_finality(&self) -> StmDynResult<IPCParentFinality> {
        let finality = self.last_committed_finality.read_clone()?;
        Ok(finality)
    }

    fn next_proposal(&self) -> StmDynResult<Option<IPCParentFinality>> {
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
        let last_committed_finality = self.last_committed_finality.read()?;

        // parent height is not ready to be proposed yet
        if height <= last_committed_finality.height {
            return Ok(None);
        }

        // prepare block hash and validator set
        let height_data = self.parent_view_data.height_data.read()?;
        let (block_hash, validator_set) = if let Some(v) = height_data.get_value(height) {
            let (block_hash, validator_set, _) = v;
            (block_hash.clone(), validator_set.clone())
        } else {
            return abort(Error::HeightNotFoundInCache(height));
        };

        // prepare top down messages
        let top_down_msgs = self
            .parent_view_data
            .top_down_msgs(last_committed_finality.height + 1, height)?;

        // prepare top down message nonce
        let next_top_down_nonce = if top_down_msgs.is_empty() {
            last_committed_finality.next_top_down_nonce
        } else {
            top_down_msgs.last().unwrap().msg.nonce + 1
        };

        Ok(Some(IPCParentFinality {
            height,
            block_hash,
            top_down_msgs,
            next_top_down_nonce,
            validator_set,
        }))
    }

    fn check_proposal(&self, proposal: &IPCParentFinality) -> StmDynResult<()> {
        self.check_height(proposal)?;
        self.check_block_hash(proposal)?;
        self.check_validator_set(proposal)?;
        self.check_top_down_msgs(proposal)
    }

    fn on_finality_committed(&self, finality: &IPCParentFinality) -> StmDynResult<()> {
        // the height to clear
        let height = finality.height;

        let cache_empty = self.parent_view_data.height_data.modify(|mut cache| {
            cache.remove_key_below(height + 1);
            let cache_empty = cache.is_empty();
            (cache, cache_empty)
        })?;

        let parent_view_next_nonce = *self.parent_view_data.next_top_down_nonce.read()?;
        if cache_empty {
            // with the new finality, the parent view cache is all cleared, this means there is
            // no top down messages in the parent view cache. We set the next top down nonce to be
            // that in the current finality.
            debug_assert!(
                finality.next_top_down_nonce >= parent_view_next_nonce,
                "finality's next nonce should not be smaller than that in the parent view"
            );
            self.parent_view_data
                .next_top_down_nonce
                .write(finality.next_top_down_nonce)?;
        } else {
            // the parent view cache is not all cleared, this means there might be some top down
            // messages in the parent view cache. This means the next nonce in the incoming finality
            // should not be larger than that in the parent view.
            debug_assert!(
                finality.next_top_down_nonce <= parent_view_next_nonce,
                "finality's next nonce should not be greater than that in the parent view"
            );
        }

        self.last_committed_finality.write(finality.clone())?;

        Ok(())
    }
}

impl DefaultFinalityProvider {
    pub fn new(config: Config, committed_finality: IPCParentFinality) -> Self {
        let height_data = SequentialKeyCache::new(config.block_interval);

        Self {
            config,
            parent_view_data: ParentViewData {
                height_data: TVar::new(height_data),
                next_top_down_nonce: TVar::new(committed_finality.next_top_down_nonce),
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
        if proposal.top_down_msgs.is_empty() {
            // top down msgs are empty, no need checks
            return Ok(());
        }

        ensure_sequential_by_nonce(&proposal.top_down_msgs)?;

        let proposal_min_nonce = proposal.top_down_msgs.first().unwrap().msg.nonce;
        let next_nonce = self.last_committed_finality.read()?.next_top_down_nonce;

        if next_nonce != proposal_min_nonce {
            return abort(Error::InvalidNonce {
                proposal: proposal_min_nonce,
                parent: next_nonce,
                block: proposal.height,
            });
        }

        Ok(())
    }
}

fn ensure_sequential_by_nonce(msgs: &[CrossMsg]) -> StmDynResult<()> {
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
    use crate::error::Error;
    use crate::{
        Config, DefaultFinalityProvider, IPCParentFinality, Nonce, ParentFinalityProvider,
        ParentViewProvider,
    };
    use async_stm::{atomically_or_err, StmDynError};
    use fvm_shared::address::Address;
    use fvm_shared::econ::TokenAmount;
    use ipc_sdk::cross::{CrossMsg, StorableMsg};
    use ipc_sdk::subnet_id::SubnetID;
    use ipc_sdk::ValidatorSet;

    macro_rules! downcast_err {
        ($r:ident) => {
            match $r {
                Ok(v) => Ok(v),
                Err(e) => match e {
                    StmDynError::Abort(e) => match e.downcast_ref::<Error>() {
                        None => unreachable!(),
                        Some(e) => Err(e.clone()),
                    },
                    _ => unreachable!(),
                },
            }
        };
    }

    fn new_provider() -> DefaultFinalityProvider {
        let config = Config {
            chain_head_delay: 20,
            block_interval: 1,
            polling_interval_secs: 10,
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
            top_down_msgs: vec![],
            next_top_down_nonce: 0,
            validator_set: Default::default(),
        };

        DefaultFinalityProvider::new(config, genesis_finality)
    }

    fn new_cross_msg(nonce: Nonce) -> CrossMsg {
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
            let r = provider.next_proposal();
            assert!(r.is_err());
            assert_eq!(downcast_err!(r).unwrap_err(), Error::HeightNotReady);

            provider.new_parent_view(10, vec![1u8; 32], ValidatorSet::new(vec![], 10), vec![])?;

            let r = provider.next_proposal();
            assert!(r.is_err());
            assert_eq!(
                downcast_err!(r).unwrap_err(),
                Error::HeightThresholdNotReached
            );

            // inject data
            for i in 11..=100 {
                provider.new_parent_view(i, vec![1u8; 32], ValidatorSet::new(vec![], i), vec![])?;
            }

            let proposal = provider.next_proposal()?.unwrap();
            let target_block = 100 - 20; // deduct chain head delay
            assert_eq!(
                proposal,
                IPCParentFinality {
                    height: target_block,
                    block_hash: vec![1u8; 32],
                    top_down_msgs: vec![],
                    next_top_down_nonce: 0,
                    validator_set: ValidatorSet::new(vec![], target_block),
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
                provider.new_parent_view(i, vec![1u8; 32], ValidatorSet::new(vec![], i), vec![])?;
            }

            let target_block = 120;
            let finality = IPCParentFinality {
                height: target_block,
                block_hash: vec![1u8; 32],
                top_down_msgs: vec![],
                next_top_down_nonce: 0,
                validator_set: ValidatorSet::new(vec![], target_block),
            };
            provider.on_finality_committed(&finality)?;

            // all cache should be cleared
            let r = provider.next_proposal();
            assert!(r.is_err());
            assert_eq!(downcast_err!(r).unwrap_err(), Error::HeightNotReady);

            let f = provider.last_committed_finality()?;
            assert_eq!(f, finality);

            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_check_proposal_works() {
        let provider = new_provider();

        atomically_or_err(|| {
            // inject data
            for i in 20..=100 {
                provider.new_parent_view(i, vec![1u8; 32], ValidatorSet::new(vec![], i), vec![])?;
            }

            let target_block = 120;
            let finality = IPCParentFinality {
                height: target_block,
                block_hash: vec![1u8; 32],
                top_down_msgs: vec![],
                next_top_down_nonce: 0,
                validator_set: ValidatorSet::new(vec![], target_block),
            };

            let r = provider.check_proposal(&finality);
            assert!(r.is_err());
            assert_eq!(
                downcast_err!(r).unwrap_err(),
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
                next_top_down_nonce: 0,
                validator_set: ValidatorSet::new(vec![], target_block),
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
            block_interval: 1,
            polling_interval_secs: 10,
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
            top_down_msgs: vec![],
            next_top_down_nonce: 0,
            validator_set: Default::default(),
        };

        let provider = DefaultFinalityProvider::new(config, genesis_finality);

        let cross_msgs_batch1 = vec![new_cross_msg(0), new_cross_msg(1), new_cross_msg(2)];
        let cross_msgs_batch2 = vec![new_cross_msg(3), new_cross_msg(4), new_cross_msg(5)];
        let cross_msgs_batch3 = vec![new_cross_msg(6), new_cross_msg(7), new_cross_msg(8)];
        let cross_msgs_batch4 = vec![new_cross_msg(9), new_cross_msg(10), new_cross_msg(11)];

        atomically_or_err(|| {
            provider.new_parent_view(
                100,
                vec![1u8; 32],
                ValidatorSet::new(vec![], 0),
                cross_msgs_batch1.clone(),
            )?;

            assert_eq!(provider.next_nonce()?, 3);

            provider.new_parent_view(
                101,
                vec![1u8; 32],
                ValidatorSet::new(vec![], 0),
                cross_msgs_batch2.clone(),
            )?;
            assert_eq!(provider.next_nonce()?, 6);

            provider.new_parent_view(
                102,
                vec![1u8; 32],
                ValidatorSet::new(vec![], 0),
                cross_msgs_batch3.clone(),
            )?;
            provider.new_parent_view(
                103,
                vec![1u8; 32],
                ValidatorSet::new(vec![], 0),
                cross_msgs_batch4.clone(),
            )?;
            assert_eq!(provider.next_nonce()?, 12);

            let mut v1 = cross_msgs_batch1.clone();
            let v2 = cross_msgs_batch2.clone();
            v1.extend(v2);
            let finality = IPCParentFinality {
                height: 101,
                block_hash: vec![1u8; 32],
                top_down_msgs: v1,
                next_top_down_nonce: 6,
                validator_set: ValidatorSet::new(vec![], 0),
            };
            let next_proposal = provider.next_proposal()?.unwrap();
            assert_eq!(next_proposal, finality);

            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_top_down_msgs_nonce_invalid_works() {
        let config = Config {
            chain_head_delay: 2,
            block_interval: 1,
            polling_interval_secs: 10,
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
            top_down_msgs: vec![],
            next_top_down_nonce: 0,
            validator_set: Default::default(),
        };

        let provider = DefaultFinalityProvider::new(config, genesis_finality);

        let cross_msgs_batch1 = vec![new_cross_msg(0), new_cross_msg(1), new_cross_msg(2)];
        let cross_msgs_batch3 = vec![new_cross_msg(6), new_cross_msg(7), new_cross_msg(8)];

        atomically_or_err(|| {
            provider.new_parent_view(
                100,
                vec![1u8; 32],
                ValidatorSet::new(vec![], 0),
                cross_msgs_batch1.clone(),
            )?;

            assert_eq!(provider.next_nonce()?, 3);

            let r = provider.new_parent_view(
                101,
                vec![1u8; 32],
                ValidatorSet::new(vec![], 0),
                cross_msgs_batch3.clone(),
            );
            assert!(r.is_err());
            assert_eq!(downcast_err!(r).unwrap_err(), Error::NonceNotSequential);
            assert_eq!(provider.next_nonce()?, 3);

            Ok(())
        })
        .await
        .unwrap();
    }
}
