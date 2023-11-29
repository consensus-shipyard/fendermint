// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

mod fetch;
mod null;

use crate::error::Error;
use crate::BlockHash;
use async_stm::{abort, StmResult};
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::staking::StakingChangeRequest;

pub use fetch::CachedFinalityProvider;
pub use null::FinalityWithNull;

pub(crate) type ParentViewPayload = (BlockHash, Vec<StakingChangeRequest>, Vec<CrossMsg>);

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

pub(crate) fn validator_changes(p: &ParentViewPayload) -> Vec<StakingChangeRequest> {
    p.1.clone()
}

pub(crate) fn topdown_cross_msgs(p: &ParentViewPayload) -> Vec<CrossMsg> {
    p.2.clone()
}

#[cfg(test)]
mod tests {
    use crate::proxy::ParentQueryProxy;
    use crate::{
        BlockHeight, CachedFinalityProvider, Config, IPCParentFinality, ParentFinalityProvider,
    };
    use async_stm::atomically_or_err;
    use async_trait::async_trait;
    use fvm_shared::address::Address;
    use fvm_shared::econ::TokenAmount;
    use ipc_provider::manager::{GetBlockHashResult, TopDownQueryPayload};
    use ipc_sdk::cross::{CrossMsg, StorableMsg};
    use ipc_sdk::staking::StakingChangeRequest;
    use ipc_sdk::subnet_id::SubnetID;
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

        async fn get_top_down_msgs(
            &self,
            _height: BlockHeight,
        ) -> anyhow::Result<TopDownQueryPayload<Vec<CrossMsg>>> {
            Ok(TopDownQueryPayload {
                value: vec![],
                block_hash: vec![],
            })
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
            max_proposal_range: None,
            max_cache_blocks: None,
            proposal_delay: None,
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
    async fn test_next_proposal_works() {
        let min_proposal_interval = 10;
        let config = Config {
            chain_head_delay: 2,
            polling_interval: Duration::from_secs(10),
            exponential_back_off: Duration::from_secs(10),
            exponential_retry_limit: 10,
            max_proposal_range: Some(min_proposal_interval),
            max_cache_blocks: None,
            proposal_delay: None,
        };

        let last_committed_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        };

        let provider = CachedFinalityProvider::new(
            config,
            10,
            Some(last_committed_finality),
            mocked_agent_proxy(),
        );

        atomically_or_err(|| {
            for h in 1..20 {
                provider.new_parent_view(
                    h,
                    Some((vec![1u8; 32], vec![], vec![new_cross_msg(h - 1)])),
                )?;
            }

            let finality = IPCParentFinality {
                height: min_proposal_interval,
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
    async fn test_nex_proposal_null_blocks_works() {
        let min_proposal_interval = 10;
        let config = Config {
            chain_head_delay: 2,
            polling_interval: Duration::from_secs(10),
            exponential_back_off: Duration::from_secs(10),
            exponential_retry_limit: 10,
            max_proposal_range: Some(min_proposal_interval),
            max_cache_blocks: None,
            proposal_delay: None,
        };

        let last_committed_finality = IPCParentFinality {
            height: 0,
            block_hash: vec![0; 32],
        };

        let provider = CachedFinalityProvider::new(
            config,
            10,
            Some(last_committed_finality),
            mocked_agent_proxy(),
        );

        atomically_or_err(|| {
            for h in 1..20 {
                provider.new_parent_view(h, None)?;
            }

            provider.new_parent_view(20, Some((vec![1u8; 32], vec![], vec![new_cross_msg(0)])))?;

            let finality = IPCParentFinality {
                height: 20,
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
