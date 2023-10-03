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

use arbitrary::{Arbitrary, Unstructured};
use fendermint_testing::{smt::StateMachine, state_machine_test};
use fendermint_vm_genesis::Genesis;
use fendermint_vm_interpreter::fvm::{
    state::{ipc::GatewayCaller, FvmExecState},
    store::memory::MemoryBlockstore,
};

/// System Under Test for staking.
struct StakingSystem {
    /// FVM state initialized with the parent genesis, and a subnet created for the child.
    parent_state: FvmExecState<MemoryBlockstore>,
    /// FVM state initialized with the child genesis.
    child_state: FvmExecState<MemoryBlockstore>,
    gateway: GatewayCaller<MemoryBlockstore>,
}

/// Reference implementation for staking.
#[derive(Debug, Clone)]
struct StakingState {
    /// The parent genesis should include a bunch of accounts we can use to join a subnet.
    parent_genesis: Genesis,
    /// The child genesis should start with a subset of the parent accounts being validators.
    child_genesis: Genesis,
}

impl arbitrary::Arbitrary<'_> for StakingState {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        todo!()
    }
}

struct StakingMachine;

impl StateMachine for StakingMachine {
    type System = StakingSystem;

    type State = StakingState;

    type Command = ();

    type Result = ();

    fn gen_state(&self, u: &mut Unstructured) -> arbitrary::Result<Self::State> {
        StakingState::arbitrary(u)
    }

    fn new_system(&self, state: &Self::State) -> Self::System {
        let rt = tokio::runtime::Runtime::new().expect("create tokio runtime for init");

        let (parent_state, _) = rt
            .block_on(contract_test::init_exec_state(state.parent_genesis.clone()))
            .expect("failed to init parent");

        let (child_state, _) = rt
            .block_on(contract_test::init_exec_state(state.child_genesis.clone()))
            .expect("failed to init child");

        let gateway = GatewayCaller::default();

        StakingSystem {
            parent_state,
            child_state,
            gateway,
        }
    }

    fn gen_command(
        &self,
        u: &mut Unstructured,
        state: &Self::State,
    ) -> arbitrary::Result<Self::Command> {
        Ok(())
    }

    fn run_command(&self, system: &mut Self::System, cmd: &Self::Command) -> Self::Result {
        todo!()
    }

    fn check_result(&self, cmd: &Self::Command, pre_state: &Self::State, result: &Self::Result) {
        todo!()
    }

    fn next_state(&self, cmd: &Self::Command, state: Self::State) -> Self::State {
        todo!()
    }

    fn check_system(
        &self,
        cmd: &Self::Command,
        post_state: &Self::State,
        post_system: &Self::System,
    ) {
        todo!()
    }
}

state_machine_test!(staking, 20000 ms, 100 steps, StakingMachine);
