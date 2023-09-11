// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::cache::SequentialKeyCache;
use crate::error::Error;
use crate::{
    BlockHash, BlockHeight, Config, IPCParentFinality, ParentFinalityProvider, ParentViewProvider,
};
use async_stm::{abort, StmError, StmResult, TVar};
use ipc_agent_sdk::message::ipc::ValidatorSet;
use ipc_sdk::cross::CrossMsg;

type ParentViewPayload = (BlockHash, ValidatorSet, Vec<CrossMsg>);

/// The default parent finality provider
pub struct InMemoryFinalityProvider {
    config: Config,
    parent_view_data: ParentViewData,
    /// This is a in memory view of the committed parent finality,
    /// it should be synced with the store committed finality, owner of the struct should enforce
    /// this.
    last_committed_finality: TVar<Option<IPCParentFinality>>,
}

/// Tracks the data from the parent
#[derive(Clone)]
struct ParentViewData {
    height_data: TVar<SequentialKeyCache<BlockHeight, ParentViewPayload>>,
}

impl ParentViewData {
    fn latest_height(&self) -> StmResult<Option<BlockHeight>, Error> {
        let cache = self.height_data.read()?;
        // safe to unwrap, we dont allow no upper bound
        Ok(cache.upper_bound())
    }

    fn block_hash(&self, height: BlockHeight) -> StmResult<Option<BlockHash>, Error> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).map(|i| i.0.clone()))
    }

    fn validator_set(&self, height: BlockHeight) -> StmResult<Option<ValidatorSet>, Error> {
        let cache = self.height_data.read()?;
        Ok(cache.get_value(height).map(|i| i.1.clone()))
    }

    fn top_down_msgs(
        &self,
        from_height: BlockHeight,
        to_height: BlockHeight,
    ) -> StmResult<Vec<CrossMsg>, Error> {
        let cache = self.height_data.read()?;
        let v = cache
            .values_within(from_height, to_height)
            .flat_map(|i| i.2.iter())
            .cloned()
            .collect();
        Ok(v)
    }
}

impl ParentViewProvider for InMemoryFinalityProvider {
    fn latest_height(&self) -> StmResult<Option<BlockHeight>, Error> {
        let h = self.parent_view_data.latest_height()?;
        Ok(h)
    }

    fn block_hash(&self, height: BlockHeight) -> StmResult<Option<BlockHash>, Error> {
        let v = self.parent_view_data.block_hash(height)?;
        Ok(v)
    }

    fn validator_set(&self, height: BlockHeight) -> StmResult<Option<ValidatorSet>, Error> {
        let v = self.parent_view_data.validator_set(height)?;
        Ok(v)
    }

    fn top_down_msgs(&self, height: BlockHeight) -> StmResult<Vec<CrossMsg>, Error> {
        let v = self.parent_view_data.top_down_msgs(height, height)?;
        Ok(v)
    }

    fn new_parent_view(
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

impl ParentFinalityProvider for InMemoryFinalityProvider {
    fn last_committed_finality(&self) -> StmResult<IPCParentFinality, Error> {
        let finality = self
            .last_committed_finality
            .read_clone()?
            .ok_or(StmError::Abort(Error::CommittedFinalityNotReady))?;
        Ok(finality)
    }

    fn next_proposal(&self) -> StmResult<Option<IPCParentFinality>, Error> {
        let latest_height = if let Some(h) = self.parent_view_data.latest_height()? {
            h
        } else {
            return Ok(None);
        };

        // latest height has not reached, we should wait or abort
        if latest_height < self.config.chain_head_delay {
            return Ok(None);
        }

        let height = latest_height - self.config.chain_head_delay;
        let last_committed_finality = self.last_committed_finality()?;

        // parent height is not ready to be proposed yet
        if height <= last_committed_finality.height {
            return Ok(None);
        }

        // prepare block hash and validator set
        let height_data = self.parent_view_data.height_data.read()?;
        let block_hash = if let Some(v) = height_data.get_value(height) {
            let (block_hash, _, _) = v;
            block_hash.clone()
        } else {
            return abort(Error::HeightNotFoundInCache(height));
        };

        Ok(Some(IPCParentFinality { height, block_hash }))
    }

    fn check_proposal(&self, proposal: &IPCParentFinality) -> StmResult<(), Error> {
        self.check_height(proposal)?;
        self.check_block_hash(proposal)
    }

    fn on_finality_committed(&self, finality: &IPCParentFinality) -> StmResult<(), Error> {
        // the height to clear
        let height = finality.height;

        self.parent_view_data.height_data.update(|mut cache| {
            cache.remove_key_below(height + 1);
            cache
        })?;

        self.last_committed_finality.write(Some(finality.clone()))?;

        Ok(())
    }
}

impl InMemoryFinalityProvider {
    pub fn new(config: Config, committed_finality: Option<IPCParentFinality>) -> Self {
        let height_data = SequentialKeyCache::sequential();
        Self {
            config,
            parent_view_data: ParentViewData {
                height_data: TVar::new(height_data),
            },
            last_committed_finality: TVar::new(committed_finality),
        }
    }

    fn check_height(&self, proposal: &IPCParentFinality) -> StmResult<(), Error> {
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

        let last_committed_finality = self.last_committed_finality()?;
        if proposal.height <= last_committed_finality.height {
            return abort(Error::HeightAlreadyCommitted(proposal.height));
        }
        Ok(())
    }

    fn check_block_hash(&self, proposal: &IPCParentFinality) -> StmResult<(), Error> {
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
    use crate::{
        Config, IPCParentFinality, InMemoryFinalityProvider, ParentFinalityProvider,
        ParentViewProvider,
    };
    use async_stm::atomically_or_err;
    use fvm_shared::address::Address;
    use fvm_shared::econ::TokenAmount;
    use ipc_agent_sdk::message::ipc::ValidatorSet;
    use ipc_sdk::cross::{CrossMsg, StorableMsg};
    use ipc_sdk::subnet_id::SubnetID;

    fn new_provider() -> InMemoryFinalityProvider {
        let config = Config {
            chain_head_delay: 20,
            polling_interval_secs: 10,
            ipc_agent_url: "".to_string(),
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        };

        InMemoryFinalityProvider::new(config, Some(genesis_finality))
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
            assert!(r.is_none());

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
            let target_block = 100 - 20; // deduct chain head delay
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
            provider.on_finality_committed(&finality)?;

            // all cache should be cleared
            let r = provider.next_proposal()?;
            assert!(r.is_none());

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
            provider.on_finality_committed(&IPCParentFinality {
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
        };

        let genesis_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        };

        let provider = InMemoryFinalityProvider::new(config, Some(genesis_finality));

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
                height: 101,
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
