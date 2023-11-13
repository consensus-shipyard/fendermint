// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::Path;

use anyhow::Context;
use fendermint_vm_interpreter::fvm::state::{snapshot::BlockHeight, FvmStateParams};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SnapshotManifest {
    /// Block height where the snapshot was taken.
    pub block_height: BlockHeight,
    /// Snapshot size in bytes.
    pub size: usize,
    /// Number of chunks in the snapshot.
    pub chunks: usize,
    /// The FVM parameters at the time of the snapshot,
    /// which are also in the CAR file, but it might be
    /// useful to see. It is annotated for human readability.
    pub state_params: FvmStateParams,
}

/// Save a manifest along with the other snapshot files into a snapshot specific directory.
pub fn write_manifest(
    manifest_path: impl AsRef<Path>,
    manifest: &SnapshotManifest,
) -> anyhow::Result<()> {
    let json =
        serde_json::to_string_pretty(&manifest).context("failed to convert manifest to JSON")?;

    std::fs::write(manifest_path, json).context("failed to write manifest file")?;

    Ok(())
}

#[cfg(feature = "arb")]
mod arb {
    use fendermint_testing::arb::{ArbCid, ArbTokenAmount};
    use fendermint_vm_core::{chainid, Timestamp};
    use fendermint_vm_interpreter::fvm::state::FvmStateParams;
    use fvm_shared::version::NetworkVersion;
    use quickcheck::Arbitrary;

    use super::SnapshotManifest;

    impl quickcheck::Arbitrary for SnapshotManifest {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Self {
                block_height: Arbitrary::arbitrary(g),
                size: Arbitrary::arbitrary(g),
                chunks: Arbitrary::arbitrary(g),
                state_params: FvmStateParams {
                    state_root: ArbCid::arbitrary(g).0,
                    timestamp: Timestamp(Arbitrary::arbitrary(g)),
                    network_version: NetworkVersion::MAX,
                    base_fee: ArbTokenAmount::arbitrary(g).0,
                    circ_supply: ArbTokenAmount::arbitrary(g).0,
                    chain_id: chainid::from_str_hashed(String::arbitrary(g).as_str())
                        .unwrap()
                        .into(),
                    power_scale: *g.choose(&[-1, 0, 3]).unwrap(),
                },
            }
        }
    }
}
