use crate::fvm::state::FvmStateParams;
use crate::fvm::store::ReadOnlyBlockstore;
use cid::Cid;
use fvm::state_tree::{ActorState, StateTree};
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::address::Address;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{LineWriter, Write};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub type BlockHeight = u64;

#[derive(Debug)]
pub struct Snapshot<DB> {
    /// The state tree of the current blockchain
    state_tree: StateTree<ReadOnlyBlockstore<DB>>,
    /// The latest state params of the blockchain
    state_params: FvmStateParams,
    /// The latest block height
    block_height: BlockHeight,
}

const ACTOR_STATE_PER_ROW: usize = 64;

/// Each row of the snapshot file
#[derive(Serialize, Deserialize)]
enum SnapshotRow {
    StateTree(Vec<u8>),
    StateParams(Vec<u8>),
    BlockHeight(BlockHeight),
}

impl<DB> Snapshot<DB>
where
    DB: Blockstore + Clone + 'static,
{
    pub fn new(
        store: ReadOnlyBlockstore<DB>,
        state_params: FvmStateParams,
        block_height: BlockHeight,
    ) -> anyhow::Result<Self> {
        let state_tree = StateTree::new_from_root(store, &state_params.state_root)?;
        Ok(Self {
            state_tree,
            state_params,
            block_height,
        })
    }

    /// Read the snapshot from file
    pub async fn read_from_file(_path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        todo!()
    }

    /// Write the snapshot to file
    pub fn write_to_file(&self, path: impl AsRef<PathBuf>) -> anyhow::Result<()> {
        let file = File::create(path.as_ref())?;
        let mut file = LineWriter::new(file);

        self.write_state_tree(&mut file)?;
        self.write_block_height(&mut file)?;
        self.write_to_file(&mut file)?;

        Ok(())
    }

    fn write_state_tree(&self, file: &mut impl Write) -> anyhow::Result<()> {
        let flush = |file, pairs| {
            let bytes =
                serde_json::to_vec(&SnapshotRow::StateTree(fvm_ipld_encoding::to_vec(pairs)?))?;

            file.write_all(&bytes)?;
            file.write_all(b"\n")?;

            pairs.clear();

            Ok(())
        };

        let mut pairs = Vec::with_capacity(ACTOR_STATE_PER_ROW);
        self.state_tree.for_each(|(addr, state)| {
            if pairs.len() == ACTOR_STATE_PER_ROW {
                flush(file, &mut pairs)
            } else {
                pairs.push((addr, state));
                Ok(())
            }
        })?;

        // flush again for left over pairs
        flush(file, &mut pairs)?;

        Ok(())
    }

    fn write_block_height(&self, file: &mut impl Write) -> anyhow::Result<()> {
        let bytes =
            serde_json::to_vec(&SnapshotRow::BlockHeight(self.block_height))?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;

        Ok(())
    }

    fn write_state_params(&self, file: &mut impl Write) -> anyhow::Result<()> {
        let bytes =
            serde_json::to_vec(&SnapshotRow::StateParams(fvm_ipld_encoding::to_vec(&self.state_params)?))?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;

        Ok(())
    }
}
