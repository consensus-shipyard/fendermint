// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//!
use crate::fvm::state::FvmStateParams;
use crate::fvm::store::ReadOnlyBlockstore;
use cid::multihash::{Code, MultihashDigest};
use cid::Cid;
use futures_core::Stream;
use fvm::state_tree::StateTree;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::CarHeader;
use fvm_ipld_encoding::{from_slice, DAG_CBOR};

use libipld::Ipld;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

pub type BlockHeight = u64;

/// Taking snapshot of the current blockchain state
pub enum Snapshot<DB> {
    V1(V1Snapshot<DB>),
}

/// The block height with state paramsThe block height with state params
#[derive(Serialize, Deserialize)]
struct BlockStateParams {
    /// The latest state params of the blockchain
    state_params: FvmStateParams,
    /// The latest block height
    block_height: BlockHeight,
}

impl<DB> Snapshot<DB>
where
    DB: Blockstore + Clone + 'static + Send,
{
    pub fn new(
        store: ReadOnlyBlockstore<DB>,
        state_params: FvmStateParams,
        block_height: BlockHeight,
    ) -> anyhow::Result<Self> {
        Ok(Self::V1(V1Snapshot::new(
            store,
            state_params,
            block_height,
        )?))
    }

    pub fn version(&self) -> u64 {
        match self {
            Snapshot::V1(_) => 1,
        }
    }

    /// Init the block chain from the snapshot
    pub fn init_chain(&self) -> anyhow::Result<()> {
        todo!()
    }

    /// Read the snapshot from file
    pub async fn read_car(_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        todo!()
    }

    /// Write the snapshot to car file
    pub async fn write_car(self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let file = tokio::fs::File::create(path).await?;

        // derive the car header roots
        let car = CarHeader::new(self.car_header_roots()?, self.version());

        match self {
            Snapshot::V1(s) => s.write_car(car, file).await?,
        }

        Ok(())
    }

    fn car_header_roots(&self) -> anyhow::Result<Vec<Cid>> {
        match self {
            Snapshot::V1(s) => s.car_header_roots(),
        }
    }
}

pub struct V1Snapshot<DB> {
    /// The root cid of the state tree
    state_tree_root: Cid,
    /// The state tree of the current blockchain
    state_tree: StateTree<ReadOnlyBlockstore<DB>>,
    /// The latest block height with state params serialized to bytes
    block_state_params_bytes: Vec<u8>,
    /// Block state params cid derived from block state params bytes
    block_state_params_cid: Cid,
}

impl<DB> V1Snapshot<DB>
where
    DB: Blockstore + Clone + 'static + Send,
{
    /// Creates a new V2Snapshot struct. Caller ensure store
    pub fn new(
        store: ReadOnlyBlockstore<DB>,
        state_params: FvmStateParams,
        block_height: BlockHeight,
    ) -> anyhow::Result<Self> {
        let state_tree = StateTree::new_from_root(store, &state_params.state_root)?;
        let state_tree_root = state_params.state_root;

        let block_state_params = BlockStateParams {
            state_params,
            block_height,
        };
        let block_state_params_bytes = fvm_ipld_encoding::to_vec(&block_state_params)?;
        let block_state_params_cid =
            Cid::new_v1(DAG_CBOR, Code::Blake2b256.digest(&block_state_params_bytes));

        Ok(Self {
            state_tree_root,
            state_tree,
            block_state_params_bytes,
            block_state_params_cid,
        })
    }

    /// For V1 snapshot, we are putting two components into the CAR file: the state tree and latest
    /// state params. One header is for the state tree root. The other header root cid is for the
    /// serialized block state params.
    fn car_header_roots(&self) -> anyhow::Result<Vec<Cid>> {
        Ok(vec![self.state_tree_root, self.block_state_params_cid])
    }

    pub fn init_chain(&self) -> anyhow::Result<()> {
        todo!()
    }

    async fn write_car(self, car: CarHeader, write: tokio::fs::File) -> anyhow::Result<()> {
        let write_task = tokio::spawn(async move {
            let mut streamer =
                StateTreeStreamer::new(self.state_tree_root, self.state_tree.into_store());
            let mut write = AsyncWriteWrapper { w: write };

            car.write_stream_async(&mut Pin::new(&mut write), &mut streamer)
                .await
                .unwrap();

            let mut streamer = tokio_stream::iter(vec![(
                self.block_state_params_cid,
                self.block_state_params_bytes,
            )]);
            car.write_stream_async(&mut Pin::new(&mut write), &mut streamer)
                .await
                .unwrap();
        });

        write_task.await?;

        Ok(())
    }
}

#[pin_project::pin_project]
struct StateTreeStreamer<BlockStore> {
    /// The list of cids to pull from the blockstore
    #[pin]
    dfs: VecDeque<Cid>,
    /// The block store
    bs: BlockStore,
}

impl<BlockStore> StateTreeStreamer<BlockStore> {
    pub fn new(state_root_cid: Cid, bs: BlockStore) -> Self {
        let mut dfs = VecDeque::new();
        dfs.push_back(state_root_cid);
        Self { dfs, bs }
    }
}

impl<BlockStore: Blockstore> Stream for StateTreeStreamer<BlockStore> {
    type Item = (Cid, Vec<u8>);

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let cid = if let Some(cid) = this.dfs.pop_front() {
                cid
            } else {
                return Poll::Ready(None);
            };

            match this.bs.get(&cid) {
                Ok(Some(bytes)) => {
                    let ipld = from_slice::<Ipld>(&bytes).unwrap();
                    walk_ipld_cids(ipld, &mut this.dfs);
                    return Poll::Ready(Some((cid, bytes)));
                }
                Ok(None) => {
                    tracing::warn!("cid: {cid:?} has no value in block store, skip");
                    continue;
                }
                Err(e) => {
                    tracing::warn!("cannot get from block store: {}", e.to_string());
                    // TODO: consider returning Result, but it won't work with `car.write_stream_async`.
                    return Poll::Ready(None);
                }
            }
        }
    }
}

fn walk_ipld_cids(ipld: Ipld, dfs: &mut VecDeque<Cid>) {
    match ipld {
        Ipld::List(v) => {
            for i in v {
                walk_ipld_cids(i, dfs);
            }
        }
        Ipld::Map(map) => {
            for v in map.into_values() {
                walk_ipld_cids(v, dfs);
            }
        }
        Ipld::Link(cid) => dfs.push_back(cid),
        _ => {}
    }
}

/// We need this wrapper to be compatible with the current version of CarHeader.
#[pin_project::pin_project]
struct AsyncWriteWrapper<W: AsyncWrite> {
    #[pin]
    w: W,
}

impl<W: AsyncWrite> futures_util::AsyncWrite for AsyncWriteWrapper<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.project();
        this.w.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.project();
        this.w.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.project();
        this.w.poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use crate::fvm::state::snapshot::StateTreeStreamer;
    use crate::fvm::state::{FvmStateParams, Snapshot};
    use crate::fvm::store::memory::MemoryBlockstore;
    use crate::fvm::store::ReadOnlyBlockstore;
    use cid::Cid;
    use fendermint_vm_core::Timestamp;
    use futures_util::StreamExt;
    use fvm::state_tree::{ActorState, StateTree};
    use fvm_ipld_blockstore::Blockstore;
    use fvm_shared::state::StateTreeVersion;
    use fvm_shared::version::NetworkVersion;
    use quickcheck::{Arbitrary, Gen};
    use std::collections::VecDeque;

    fn prepare_state_tree(items: u64) -> (Cid, StateTree<MemoryBlockstore>) {
        let store = MemoryBlockstore::new();
        let mut state_tree = StateTree::new(store, StateTreeVersion::V5).unwrap();
        let mut gen = Gen::new(16);

        for i in 1..=items {
            let state = ActorState::arbitrary(&mut gen);
            state_tree.set_actor(i, state);
        }
        let root_cid = state_tree.flush().unwrap();
        (root_cid, state_tree)
    }

    fn assert_tree2_contains_tree1(
        tree1: &StateTree<MemoryBlockstore>,
        tree2: &StateTree<MemoryBlockstore>,
    ) {
        tree1
            .for_each(|addr, state| {
                let r = tree2.get_actor_by_address(&addr);
                if r.is_err() {
                    panic!("addr: {addr:?} does not exists in tree 2");
                }

                if let Some(target_state) = r.unwrap() {
                    assert_eq!(target_state, *state);
                } else {
                    panic!("missing address: {addr:?}");
                }
                Ok(())
            })
            .unwrap();
    }

    #[tokio::test]
    async fn test_streamer() {
        let (root_cid, state_tree) = prepare_state_tree(100);
        let bs = state_tree.into_store();
        let mut stream = StateTreeStreamer {
            dfs: VecDeque::from(vec![root_cid.clone()]),
            bs: bs.clone(),
        };

        let new_bs = MemoryBlockstore::new();
        while let Some((cid, bytes)) = stream.next().await {
            new_bs.put_keyed(&cid, &bytes).unwrap();
        }

        let new_state_tree = StateTree::new_from_root(new_bs, &root_cid).unwrap();
        let old_state_tree = StateTree::new_from_root(bs, &root_cid).unwrap();

        assert_tree2_contains_tree1(&old_state_tree, &new_state_tree);
        assert_tree2_contains_tree1(&new_state_tree, &old_state_tree);
    }

    #[tokio::test]
    async fn test_write_to_car() {
        let (state_root, state_tree) = prepare_state_tree(100);
        let state_params = FvmStateParams {
            state_root,
            timestamp: Timestamp(100),
            network_version: NetworkVersion::V1,
            base_fee: Default::default(),
            circ_supply: Default::default(),
            chain_id: 1024,
        };
        let block_height = 2048;

        let bs = state_tree.into_store();
        let db = ReadOnlyBlockstore::new(bs);
        let snapshot = Snapshot::new(db, state_params, block_height).unwrap();

        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        let r = snapshot.write_car(tmp_file.path()).await;
        assert!(r.is_ok());
    }
}
