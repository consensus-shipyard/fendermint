// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::finality::{
    ensure_sequential, topdown_cross_msgs, validator_changes, ParentViewPayload,
};
use crate::{BlockHash, BlockHeight, Error, IPCParentFinality, SequentialKeyCache};
use async_stm::{abort, atomically, Stm, StmResult, TVar};
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::staking::StakingChangeRequest;

/// Finality provider that can handle null blocks
#[derive(Clone)]
pub struct FinalityWithNull {
    /// The min topdown proposal height interval
    min_proposal_interval: BlockHeight,
    genesis_epoch: BlockHeight,
    /// Cached data that always syncs with the latest parent chain proactively
    cached_data: TVar<SequentialKeyCache<BlockHeight, Option<ParentViewPayload>>>,
    /// This is a in memory view of the committed parent finality. We need this as a starting point
    /// for populating the cache
    last_committed_finality: TVar<Option<IPCParentFinality>>,
}

impl FinalityWithNull {
    pub fn new(
        min_proposal_interval: BlockHeight,
        genesis_epoch: BlockHeight,
        committed_finality: Option<IPCParentFinality>,
    ) -> Self {
        Self {
            min_proposal_interval,
            genesis_epoch,
            cached_data: TVar::new(SequentialKeyCache::sequential()),
            last_committed_finality: TVar::new(committed_finality),
        }
    }

    pub fn genesis_epoch(&self) -> anyhow::Result<BlockHeight> {
        Ok(self.genesis_epoch)
    }

    pub async fn validator_changes(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<Option<Vec<StakingChangeRequest>>> {
        let r = atomically(|| self.handle_null_block(height, validator_changes, Vec::new)).await;
        Ok(r)
    }

    pub async fn top_down_msgs(
        &self,
        height: BlockHeight,
    ) -> anyhow::Result<Option<Vec<CrossMsg>>> {
        let r = atomically(|| self.handle_null_block(height, topdown_cross_msgs, Vec::new)).await;
        Ok(r)
    }

    pub fn last_committed_finality(&self) -> Stm<Option<IPCParentFinality>> {
        self.last_committed_finality.read_clone()
    }

    /// Clear the cache and set the committed finality to the provided value
    pub fn reset(&self, finality: IPCParentFinality) -> Stm<()> {
        self.cached_data.write(SequentialKeyCache::sequential())?;
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

    pub fn next_proposal(&self) -> Stm<Option<IPCParentFinality>> {
        let height = if let Some(h) = self.propose_next_height()? {
            h
        } else {
            return Ok(None);
        };

        // safe to unwrap as we make sure null height will not be proposed
        let block_hash = self.block_hash_at_height(height)?.unwrap();

        let proposal = IPCParentFinality { height, block_hash };
        tracing::debug!(proposal = proposal.to_string(), "new proposal");
        Ok(Some(proposal))
    }

    pub fn check_proposal(&self, proposal: &IPCParentFinality) -> Stm<bool> {
        if !self.check_height(proposal)? {
            return Ok(false);
        }
        self.check_block_hash(proposal)
    }

    pub fn set_new_finality(
        &self,
        finality: IPCParentFinality,
        previous_finality: Option<IPCParentFinality>,
    ) -> Stm<()> {
        debug_assert!(previous_finality == self.last_committed_finality.read_clone()?);

        // the height to clear
        let height = finality.height;

        self.cached_data.update(|mut cache| {
            cache.remove_key_below(height + 1);
            cache
        })?;

        self.last_committed_finality.write(Some(finality))
    }
}

impl FinalityWithNull {
    /// Returns the number of blocks cached.
    pub(crate) fn cached_blocks(&self) -> Stm<BlockHeight> {
        let cache = self.cached_data.read()?;
        Ok(cache.size() as BlockHeight)
    }

    pub(crate) fn block_hash_at_height(&self, height: BlockHeight) -> Stm<Option<BlockHash>> {
        if let Some(f) = self.last_committed_finality.read()?.as_ref() {
            if f.height == height {
                return Ok(Some(f.block_hash.clone()));
            }
        }

        self.get_at_height(height, |i| i.0.clone())
    }

    pub(crate) fn latest_height_in_cache(&self) -> Stm<Option<BlockHeight>> {
        let cache = self.cached_data.read()?;
        Ok(cache.upper_bound())
    }

    /// Get the latest height tracked in the provider, includes both cache and last committed finality
    pub(crate) fn latest_height(&self) -> Stm<Option<BlockHeight>> {
        let h = if let Some(h) = self.latest_height_in_cache()? {
            h
        } else if let Some(p) = self.last_committed_finality()? {
            p.height
        } else {
            return Ok(None);
        };
        Ok(Some(h))
    }
}

/// All the private functions
impl FinalityWithNull {
    /// Get the first non-null block in the range [start, end].
    fn min_nonnull_block(&self, start: BlockHeight, end: BlockHeight) -> Stm<Option<BlockHeight>> {
        let cache = self.cached_data.read()?;
        for h in start..=end {
            if let Some(Some(_)) = cache.get_value(h) {
                return Ok(Some(h));
            }
        }
        Ok(None)
    }

    fn propose_next_height(&self) -> Stm<Option<BlockHeight>> {
        let latest_height = if let Some(h) = self.latest_height_in_cache()? {
            h
        } else {
            tracing::debug!("no proposal yet as height not available");
            return Ok(None);
        };

        let last_committed_height = if let Some(h) = self.last_committed_finality.read_clone()? {
            h.height
        } else {
            unreachable!("last committed finality will be available at this point");
        };
        let next_proposal_height = last_committed_height + self.min_proposal_interval;

        if next_proposal_height > latest_height {
            tracing::debug!("proposal period not reached yet");
            return Ok(None);
        }

        // safe to unwrap as we are sure `latest_height` will not be null block
        Ok(Some(
            self.min_nonnull_block(next_proposal_height, latest_height)?
                .unwrap(),
        ))
    }

    fn handle_null_block<T, F: Fn(&ParentViewPayload) -> T, D: Fn() -> T>(
        &self,
        height: BlockHeight,
        f: F,
        d: D,
    ) -> Stm<Option<T>> {
        let cache = self.cached_data.read()?;
        Ok(cache.get_value(height).map(|v| {
            if let Some(i) = v.as_ref() {
                f(i)
            } else {
                tracing::debug!(height, "a null round detected, return default");
                d()
            }
        }))
    }

    fn get_at_height<T, F: Fn(&ParentViewPayload) -> T>(
        &self,
        height: BlockHeight,
        f: F,
    ) -> Stm<Option<T>> {
        let cache = self.cached_data.read()?;
        Ok(if let Some(Some(v)) = cache.get_value(height) {
            Some(f(v))
        } else {
            None
        })
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
            tracing::debug!(?top_down_msgs);
            ensure_sequential(&top_down_msgs, |msg| msg.msg.nonce)?;
        };
        if !validator_changes.is_empty() {
            tracing::debug!(?validator_changes, "validator changes");
            ensure_sequential(&validator_changes, |change| change.configuration_number)?;
        }

        let r = self.cached_data.modify(|mut cache| {
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
        let r = self.cached_data.modify(|mut cache| {
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
            tracing::debug!(
                last_committed = last_committed_finality.height,
                proposed = proposal.height,
                "proposed height already committed",
            );
            return Ok(false);
        }

        if let Some(latest_height) = self.latest_height_in_cache()? {
            let r = latest_height >= proposal.height;
            tracing::debug!(is_true = r, "incoming proposal height seen?");
            // requires the incoming height cannot be more advanced than our trusted parent node
            Ok(r)
        } else {
            // latest height is not found, meaning we dont have any prefetched cache, we just be
            // strict and vote no simply because we don't know.
            tracing::debug!("reject proposal, no data in cache");
            Ok(false)
        }
    }

    fn check_block_hash(&self, proposal: &IPCParentFinality) -> Stm<bool> {
        Ok(
            if let Some(block_hash) = self.block_hash_at_height(proposal.height)? {
                let r = block_hash == proposal.block_hash;
                tracing::debug!(proposal = proposal.to_string(), is_same = r, "same hash?");
                r
            } else {
                tracing::debug!(proposal = proposal.to_string(), "reject, hash not found");
                false
            },
        )
    }
}
