use crate::cache::SequentialKeyCache;
use crate::error::Error;
use crate::{BlockHeight, Bytes, Config, IPCParentFinality, Nonce, ParentFinalityProvider};
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
    fn latest_height(&self) -> StmResult<BlockHeight> {
        let cache = self.height_data.read()?;
        // safe to unwrap, we dont allow no upper bound
        Ok(cache.upper_bound().unwrap())
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
impl ParentFinalityProvider for DefaultFinalityProvider {
    async fn last_committed_finality(&self) -> Result<IPCParentFinality, Error> {
        Ok(atomically(|| {
            let finality = self.last_committed_finality.read_clone()?;
            Ok(finality)
        })
        .await)
    }

    async fn next_proposal(&self) -> Result<IPCParentFinality, Error> {
        atomically(|| {
            let latest_height = self.parent_view_data.latest_height()?;

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

    async fn on_finality_committed(&self, finality: &IPCParentFinality) -> Result<(), Error> {
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
        })
        .await;

        Ok(())
    }
}

impl DefaultFinalityProvider {
    fn check_height(&self, proposal: &IPCParentFinality) -> StmResult<Result<(), Error>> {
        let latest_height = self.parent_view_data.latest_height()?;
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
