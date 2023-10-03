// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! State Machine Test for the Staking contracts.
//!
//! The test simulates random actions validators can take, such as depositing and withdrawing
//! collateral, and executes these actions on the actual Solidity contracts as well as an
//! idealised model, comparing the results and testing that invariants are maintained.
//!
//! It can be executed the following way:
//!
//! ```text
//! cargo test --release -p contract-test --test staking
//! ```

use fendermint_vm_genesis::{ipc::IpcParams, Genesis};
use fendermint_vm_interpreter::fvm::state::ipc::GatewayCaller;
use quickcheck::Arbitrary;

// TODO: Map Arbitrary to Unstructured
// TODO: Create a system for the parent and another for the child.
// TODO: Create a StateMachine for testing a parent and child pair.
#[test]
fn probe() {
    let mut g = quickcheck::Gen::new(5);

    let mut genesis = Genesis::arbitrary(&mut g);
    // Make sure we have IPC enabled.
    genesis.ipc = Some(IpcParams::arbitrary(&mut g));

    // The only async part in this test should be the initialization.
    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime for init");
    let (mut state, _out) = rt
        .block_on(contract_test::init_exec_state(genesis))
        .expect("genesis initialized");

    let gateway = GatewayCaller::default();

    // Some dummy test.
    let period = gateway
        .bottom_up_check_period(&mut state)
        .expect("IPC enabled");

    assert_ne!(period, 0);
}
