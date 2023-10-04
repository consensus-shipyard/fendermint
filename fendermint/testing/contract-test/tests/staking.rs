// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
#![allow(unused)]
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
use std::collections::HashMap;

use arbitrary::{Arbitrary, Unstructured};
use fendermint_crypto::{PublicKey, SecretKey};
use fendermint_testing::arb::{ArbSubnetID, ArbTokenAmount};
use fendermint_testing::{smt::StateMachine, state_machine_test};
use fendermint_vm_core::Timestamp;
use fendermint_vm_genesis::ipc::{GatewayParams, IpcParams};
use fendermint_vm_genesis::{
    Account, Actor, ActorMeta, Genesis, Power, SignerAddr, Validator, ValidatorKey,
};
use fendermint_vm_interpreter::fvm::{
    state::{ipc::GatewayCaller, FvmExecState},
    store::memory::MemoryBlockstore,
};
use fvm_shared::address::Address;
use fvm_shared::bigint::BigInt;
use fvm_shared::{econ::TokenAmount, version::NetworkVersion};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// System Under Test for staking.
struct StakingSystem {
    /// FVM state initialized with the parent genesis, and a subnet created for the child.
    parent_state: FvmExecState<MemoryBlockstore>,
    /// Facilitate calling the gateway.
    gateway: GatewayCaller<MemoryBlockstore>,
}

/// Reference implementation for staking.
#[derive(Debug, Clone)]
struct StakingState {
    /// The parent genesis should include a bunch of accounts we can use to join a subnet.
    parent_genesis: Genesis,
    /// Accounts with secret key of accounts in case the contract wants to validate signatures.
    accounts: HashMap<Address, StakingAccount>,
    child_validators: HashMap<Address, Power>,
}

impl StakingState {
    fn new(
        parent_genesis: Genesis,
        accounts: Vec<StakingAccount>,
        child_validators: Vec<(Address, Power)>,
    ) -> Self {
        Self {
            parent_genesis,
            accounts: accounts.into_iter().map(|a| (a.addr, a)).collect(),
            child_validators: child_validators.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct StakingAccount {
    public_key: PublicKey,
    secret_key: SecretKey,
    addr: Address,
    /// In this test the accounts should never gain more than their initial balance.
    initial_balance: TokenAmount,
    /// Balance after the effects of deposits/withdrawals.
    current_balance: TokenAmount,
}

impl arbitrary::Arbitrary<'_> for StakingState {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        // Limit the maximum number of *child subnet* validators to what the hypothetical consensus algorithm can scale to.
        let num_max_validators = 1 + usize::arbitrary(u)? % 10;
        // Create a number of accounts; it's okay if not everyone can become validators, and also okay if all of them can.
        let num_accounts = 1 + usize::arbitrary(u)? % 20;
        // Choose the size for the initial *child subnet* validator set.
        let num_validators = 1 + usize::arbitrary(u)? % num_accounts.min(num_max_validators);

        // Create the desired number of accounts.
        let mut rng = StdRng::seed_from_u64(u64::arbitrary(u)?);
        let mut accounts = Vec::new();
        for _ in 0..num_accounts {
            let sk = SecretKey::random(&mut rng);
            let pk = sk.public_key();
            let addr = Address::new_secp256k1(&pk.serialize()).unwrap();

            // Create with a non-zero balance so we can pick anyone to be a validator and deposit some collateral.
            let b = ArbTokenAmount::arbitrary(u)?
                .0
                .max(TokenAmount::from_atto(1));

            let a = StakingAccount {
                public_key: pk,
                secret_key: sk,
                addr,
                initial_balance: b.clone(),
                current_balance: b,
            };
            accounts.push(a);
        }

        let parent_actors = accounts
            .iter()
            .map(|s| Actor {
                meta: ActorMeta::Account(Account {
                    owner: SignerAddr(s.addr),
                }),
                balance: s.initial_balance.clone(),
            })
            .collect();

        // Select one validator to be the parent validator, it doesn't matter who.
        let parent_validators = vec![Validator {
            public_key: ValidatorKey(accounts[0].public_key),
            // All the power in the parent subnet belongs to this single validator.
            // We are only interested in the staking of the *child subnet*.
            power: Power(1),
        }];

        // Select some of the accounts to be the initial *child subnet* validators.
        let child_validators = accounts
            .iter()
            .take(num_validators)
            .map(|a| {
                // Power has to be a u64.
                let p = ArbTokenAmount::arbitrary(u)?.0.atto() % BigInt::from(u64::MAX);
                // Cannot have more than the balance.
                let p = p.max(a.initial_balance.atto().clone()).min(BigInt::from(1));
                let p = Power(p.try_into().expect("power must be u64"));
                Ok((a.addr, p))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Make sure IPC is enabled.
        let ipc = IpcParams {
            gateway: GatewayParams {
                subnet_id: ArbSubnetID::arbitrary(u)?.0,
                bottom_up_check_period: 1 + u.choose_index(100)? as u64,
                top_down_check_period: 1 + u.choose_index(100)? as u64,
                msg_fee: ArbTokenAmount::arbitrary(u)?.0,
                majority_percentage: 51 + u8::arbitrary(u)? % 50,
                min_collateral: ArbTokenAmount::arbitrary(u)?
                    .0
                    .max(TokenAmount::from_atto(1)),
            },
        };

        let g = Genesis {
            chain_name: String::arbitrary(u)?,
            timestamp: Timestamp(u64::arbitrary(u)?),
            network_version: NetworkVersion::V20,
            base_fee: ArbTokenAmount::arbitrary(u)?.0,
            validators: parent_validators,
            accounts: parent_actors,
            ipc: Some(ipc),
        };

        Ok(StakingState::new(g, accounts, child_validators))
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

        let gateway = GatewayCaller::default();

        StakingSystem {
            parent_state,
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
