// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use async_stm::{
    queues::{tchan::TChan, TQueueLike},
    StmResult, TVar,
};
use cid::Cid;
use im::hashmap::Entry;
use ipc_sdk::subnet_id::SubnetID;

/// CIDs we need to resolve from a specific source subnet, or our own.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ResolveItem {
    /// The source subnet, from which the data originated from.
    pub subnet_id: SubnetID,
    /// The root content identifier.
    pub cid: Cid,
}

/// Ongoing status of a resolution.
#[derive(Clone, Default)]
pub struct ResolveStatus {
    is_resolved: TVar<bool>,
}

impl ResolveStatus {
    pub fn is_resolved(&self) -> StmResult<bool> {
        self.is_resolved.read_clone()
    }
}

/// A data structure used to communicate resolution requirements and outcomes
/// between the resolver running in the background and the application waiting
/// for the results.
#[derive(Clone, Default)]
pub struct ResolvePool {
    /// The resolution status of each item.
    items: TVar<im::HashMap<ResolveItem, ResolveStatus>>,
    /// Items queued for resolution.
    queue: TChan<(ResolveItem, ResolveStatus)>,
}

impl ResolvePool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an item to the resolution targets.
    ///
    /// If the item is new, enqueue it from background resolution, otherwise just return its existing status.
    pub fn add(&self, item: ResolveItem) -> StmResult<ResolveStatus> {
        let (status, is_new) = self.items.modify(|mut items| {
            let ret = match items.entry(item.clone()) {
                Entry::Occupied(e) => (e.get().clone(), false),
                Entry::Vacant(e) => (e.insert(ResolveStatus::default()).clone(), true),
            };
            (items, ret)
        })?;

        if is_new {
            self.queue.write((item, status.clone()))?;
        }

        Ok(status)
    }

    /// Return the status of an item. It can be queried for completion.
    pub fn get_status(&self, item: &ResolveItem) -> StmResult<Option<ResolveStatus>> {
        Ok(self.items.read()?.get(item).cloned())
    }
}

#[cfg(test)]
mod tests {
    use async_stm::{atomically, queues::TQueueLike};
    use cid::Cid;
    use ipc_sdk::subnet_id::SubnetID;

    use super::{ResolveItem, ResolvePool};

    fn dummy_resolve_item(root_id: u64) -> ResolveItem {
        ResolveItem {
            subnet_id: SubnetID::new_root(root_id),
            cid: Cid::default(),
        }
    }

    #[tokio::test]
    async fn add_new_item() {
        let pool = ResolvePool::new();
        let item = dummy_resolve_item(0);

        atomically(|| pool.add(item.clone())).await;
        atomically(|| {
            assert!(pool.items.read()?.contains_key(&item));
            assert!(!pool.queue.is_empty()?);
            assert_eq!(pool.queue.read()?.0, item);
            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn add_existing_item() {
        let pool = ResolvePool::new();
        let item = dummy_resolve_item(0);

        // Add once.
        atomically(|| pool.add(item.clone())).await;

        // Consume it from the queue.
        atomically(|| {
            assert!(!pool.queue.is_empty()?);
            pool.queue.read()
        })
        .await;

        // Add again.
        atomically(|| pool.add(item.clone())).await;

        // Should not be queued a second time.
        atomically(|| {
            assert!(pool.items.read()?.contains_key(&item));
            assert!(pool.queue.is_empty()?);
            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn get_status() {
        let pool = ResolvePool::new();
        let item = dummy_resolve_item(0);

        let status1 = atomically(|| pool.add(item.clone())).await;
        let status2 = atomically(|| pool.get_status(&item))
            .await
            .expect("status exists");

        // Complete the item.
        atomically(|| {
            assert!(!pool.queue.is_empty()?);
            let (_, status) = pool.queue.read()?;
            status.is_resolved.write(true)
        })
        .await;

        // Check status.
        atomically(|| {
            assert!(status1.is_resolved()?);
            assert!(status2.is_resolved()?);
            Ok(())
        })
        .await;
    }
}
