// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! Interfacing with IPC, provides utility functions

mod message;
mod parent;
pub mod pof;

pub use message::IPCMessage;

#[derive(Debug, Clone)]
pub struct Config {
    /// The number of blocks to delay reporting when creating the pof
    chain_head_delay: u64,

    /// Parent syncing cron period, in seconds
    polling_interval: u64,
}
