// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use std::marker::PhantomData;

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

/// Default gas overestimation and search step
/// default value
pub const DEFAULT_GAS_RATE: f64 = 1.25;

/// Interpreter working on already verified unsigned messages.
#[derive(Clone)]
pub struct FvmMessageInterpreter<DB> {
    _phantom_db: PhantomData<DB>,
    /// Overestimation rate applied to gas to ensure that the
    /// message goes through in the gas estimation.
    gas_overestimation_rate: f64,
    /// Gas search step increase used to find the optimal gas limit.
    /// It determines how fine-grained we want the gas estimation to be.
    gas_search_step: f64,
}

impl<DB> FvmMessageInterpreter<DB> {
    pub fn new(gas_overestimation_rate: f64, gas_search_step: f64) -> Self {
        Self {
            _phantom_db: PhantomData,
            gas_overestimation_rate,
            gas_search_step,
        }
    }
}
