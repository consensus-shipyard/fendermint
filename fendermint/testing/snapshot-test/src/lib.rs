// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! Run tests against multiple Fendermint+CometBFT docker container pairs locally:
//! 0. The default `snapshot-fendermint` and `snapshot-cometbft` pair
//! 1. A `snapshot-fendermint-1` and `snapshot-cometbft-1` using `scripts/node-1.env`,
//!    which sync with the default node from genesis on a block-by-block basis.
//! 2. A `snapshot-fendermint-2` and `snapshot-cometbft-2` using `scripts/node-2.env`,
//!    which syncs with `node-0` and `node-1` using snapshots (a.k.a. state sync).
//!
//! Note that CometBFT state sync requires 2 RPC servers, which is why we need 3 nodes.
//!
//! Example:
//!
//! ```text
//! cd fendermint/testing/snapshot-test
//! cargo make
//! ```
//!
//! Make sure you installed cargo-make by running `cargo install cargo-make` first.
