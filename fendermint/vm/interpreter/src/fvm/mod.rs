// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use std::{marker::PhantomData, path::PathBuf};

mod check;
mod exec;
mod externs;
mod genesis;
mod query;
pub mod state;
mod store;

#[cfg(any(test, feature = "bundle"))]
pub mod bundle;

pub use check::FvmCheckRet;
pub use exec::FvmApplyRet;
pub use fendermint_vm_message::query::FvmQuery;
pub use genesis::FvmGenesisOutput;
pub use query::FvmQueryRet;

pub type FvmMessage = fvm_shared::message::Message;

/// Interpreter working on already verified unsigned messages.
#[derive(Clone)]
pub struct FvmMessageInterpreter<DB> {
    /// Directory containing Solidity or other contracts that
    /// need to be loaded during Genesis.
    contracts_dir: PathBuf,
    _phantom_db: PhantomData<DB>,
}

impl<DB> FvmMessageInterpreter<DB> {
    pub fn new(contracts_dir: PathBuf) -> Self {
        Self {
            contracts_dir,
            _phantom_db: PhantomData,
        }
    }
}
