// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use fendermint_vm_interpreter::fvm::state::snapshot::SnapshotVersion;

/// Possible errors with snapshots.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("incompatible snapshot version: {0}")]
    IncompatibleVersion(SnapshotVersion),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
