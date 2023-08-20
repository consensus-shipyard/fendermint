use async_stm::{atomically, StmResult, TVar};
use async_trait::async_trait;
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::ValidatorSet;
use crate::{BlockHeight, Bytes, Config, IPCParentFinality, Nonce, ParentFinalityProvider};
use crate::cache::SequentialKeyCache;

pub struct DefaultFinalityProvider {
    config: Config,
    parent_view_data: ParentViewData,
    /// The last committed finality.
    last_committed_finality: TVar<IPCParentFinality>,
}

/// Tracks the data from the parent
#[derive(Clone)]
struct ParentViewData {
    height_data: TVar<SequentialKeyCache<BlockHeight, (Bytes, ValidatorSet)>>,
    top_down_msgs: TVar<SequentialKeyCache<Nonce, CrossMsg>>,
}

impl ParentViewData {
    fn latest_height(&self) -> StmResult<BlockHeight> {
        let cache = self.height_data.read()?;
        // safe to unwrap, we dont allow no upper bound
        Ok(cache.upper_bound().unwrap())
    }

    fn all_top_down_msgs(&self) -> StmResult<Vec<CrossMsg>> {
        let cache = self.top_down_msgs.read()?;
        let start = cache.lower_bound().unwrap();
        Ok(cache.values_from(start).into_iter().cloned().collect())
    }
}

#[async_trait]
impl ParentFinalityProvider for DefaultFinalityProvider {
    async fn next_proposal(&self) -> anyhow::Result<IPCParentFinality> {
        let r = atomically(|| {
            let latest_height = self.parent_view_data.latest_height()?;

            // latest height has not reached, we should wait or abort
            if latest_height < self.config.chain_head_delay {
                // FIXME: how to handle abort correctly?
                todo!()
            }

            let height = latest_height - self.config.chain_head_delay;

            let height_data = self.parent_view_data.height_data.read()?;
            let (block_hash, validator_set) = if let Some(v) = height_data.get_value(height) {
                v.clone()
            } else {
                // FIXME: how to handle abort correctly?
                todo!()
            };

            let top_down_msgs = self.parent_view_data.all_top_down_msgs()?;

            Ok(IPCParentFinality {
                height,
                block_hash,
                top_down_msgs,
                validator_set,
            })
        }).await;
        Ok(r)
    }

    async fn check_proposal(&self, proposal: &IPCParentFinality) -> anyhow::Result<bool> {
        todo!()
    }

    async fn on_finality_committed(&self, finality: &IPCParentFinality) -> anyhow::Result<()> {
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
                cache.remove_key_till(height + 1);
                (cache, ())
            })?;

            self.parent_view_data.top_down_msgs.modify(|mut cache| {
                cache.remove_key_till(nonce + 1);
                (cache, ())
            })?;

            self.last_committed_finality.write(finality.clone())?;

            Ok(())
        }).await;

        Ok(())
    }
}
