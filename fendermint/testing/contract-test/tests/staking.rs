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
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use arbitrary::{Arbitrary, Unstructured};
use fendermint_crypto::{PublicKey, SecretKey};
use fendermint_testing::arb::{ArbSubnetAddress, ArbSubnetID, ArbTokenAmount};
use fendermint_testing::state_machine_seed;
use fendermint_testing::{smt::StateMachine, state_machine_test};
use fendermint_vm_core::Timestamp;
use fendermint_vm_genesis::ipc::{GatewayParams, IpcParams};
use fendermint_vm_genesis::{
    Account, Actor, ActorMeta, Collateral, Genesis, Power, SignerAddr, Validator, ValidatorKey,
};
use fendermint_vm_interpreter::fvm::{
    state::{ipc::GatewayCaller, FvmExecState},
    store::memory::MemoryBlockstore,
};
use fvm::engine::MultiEngine;
use fvm_shared::address::Address;
use fvm_shared::bigint::BigInt;
use fvm_shared::bigint::Integer;
use fvm_shared::{econ::TokenAmount, version::NetworkVersion};
use ipc_sdk::subnet_id::SubnetID;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// System Under Test for staking.
struct StakingSystem {
    /// FVM state initialized with the parent genesis, and a subnet created for the child.
    parent_state: FvmExecState<MemoryBlockstore>,
    /// Facilitate calling the gateway.
    gateway: GatewayCaller<MemoryBlockstore>,
}

#[derive(Debug, Clone)]
enum StakingOp {
    Deposit(TokenAmount),
    Withdraw(TokenAmount),
}

/// The staking message that goes towards the subnet to increase or decrease power.
#[derive(Debug, Clone)]
struct StakingUpdate {
    configuration_number: u64,
    addr: Address,
    op: StakingOp,
}

#[derive(Debug, Clone)]
struct StakingAccount {
    public_key: PublicKey,
    secret_key: SecretKey,
    addr: Address,
    /// In this test the accounts should never gain more than their initial balance.
    initial_balance: TokenAmount,
    /// Initial stake this account is going to put into the subnet.
    initial_stake: TokenAmount,
    /// Balance after the effects of deposits/withdrawals.
    current_balance: TokenAmount,
}

/// Reference implementation for staking.
#[derive(Debug, Clone)]
struct StakingState {
    /// Accounts with secret key of accounts in case the contract wants to validate signatures.
    accounts: HashMap<Address, StakingAccount>,
    /// List of account addresses to help pick a random one.
    addrs: Vec<Address>,
    /// The parent genesis should include a bunch of accounts we can use to join a subnet.
    parent_genesis: Genesis,
    /// The child genesis describes the initial validator set to join the subnet
    child_genesis: Genesis,
    /// Currently active child validator set.
    child_validators: HashMap<Address, Collateral>,
    /// The configuration number to be incremented before each staking operation; 0 belongs to the genesis.
    configuration_number: u64,
    /// Unconfirmed staking operations.
    pending_updates: VecDeque<StakingUpdate>,
}

impl StakingState {
    pub fn new(
        accounts: Vec<StakingAccount>,
        parent_genesis: Genesis,
        child_genesis: Genesis,
    ) -> Self {
        let child_validators = child_genesis
            .validators
            .iter()
            .map(|v| {
                let addr = Address::new_secp256k1(&v.public_key.0.serialize()).unwrap();
                (addr, v.power.clone())
            })
            .collect();

        let accounts = accounts
            .into_iter()
            .map(|a| (a.addr, a))
            .collect::<HashMap<_, _>>();

        let addrs = accounts.keys().cloned().collect();

        Self {
            accounts,
            addrs,
            parent_genesis,
            child_genesis,
            child_validators,
            configuration_number: 0,
            pending_updates: VecDeque::new(),
        }
    }

    /// Apply the changes up to `the next_configuration_number`.
    pub fn checkpoint(mut self, next_configuration_number: u64) -> Self {
        loop {
            if self.pending_updates.is_empty() {
                break;
            }
            if self.pending_updates[0].configuration_number > next_configuration_number {
                break;
            }
            let update = self.pending_updates.pop_front().expect("checked non-empty");
            match update.op {
                StakingOp::Deposit(v) => {
                    let mut power = self.child_validators.entry(update.addr).or_default();
                    power.0 += v;
                }
                StakingOp::Withdraw(v) => {
                    match self.child_validators.entry(update.addr) {
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            let c = e.get().0.clone();
                            let v = v.min(c.clone());

                            if v == c {
                                e.remove();
                            } else {
                                e.insert(Collateral(c - v.clone()));
                            }

                            let mut a = self
                                .accounts
                                .get_mut(&update.addr)
                                .expect("validators have accounts");

                            a.current_balance += v;
                        }
                        std::collections::hash_map::Entry::Vacant(_) => {
                            // Tried to withdraw more than put in.
                        }
                    }
                }
            }
        }
        self.configuration_number = next_configuration_number;
        self
    }

    /// Enqueue a deposit.
    pub fn stake(mut self, addr: Address, value: TokenAmount) -> Self {
        self.configuration_number += 1;

        let mut a = self.accounts.get_mut(&addr).expect("accounts exist");

        // Sanity check that we are generating the expected kind of values.
        // Using `debug_assert!` on the reference state to differentiate from assertions on the SUT.
        debug_assert!(
            a.current_balance >= value,
            "stakes are generated within the balance"
        );
        a.current_balance -= value.clone();

        let update = StakingUpdate {
            configuration_number: self.configuration_number,
            addr,
            op: StakingOp::Deposit(value),
        };

        self.pending_updates.push_back(update);
        self
    }

    /// Enqueue a withdrawal.
    pub fn unstake(mut self, addr: Address, value: TokenAmount) -> Self {
        self.configuration_number += 1;
        let update = StakingUpdate {
            configuration_number: self.configuration_number,
            addr,
            op: StakingOp::Withdraw(value),
        };
        self.pending_updates.push_back(update);
        self
    }
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
        for i in 0..num_accounts {
            let sk = SecretKey::random(&mut rng);
            let pk = sk.public_key();
            let addr = Address::new_secp256k1(&pk.serialize()).unwrap();

            // Create with a non-zero balance so we can pick anyone to be a validator and deposit some collateral.
            let initial_balance = ArbTokenAmount::arbitrary(u)?
                .0
                .max(TokenAmount::from_whole(1));

            // Choose an initial stake committed to the child subnet.
            let initial_stake = if i < num_validators {
                let c = BigInt::arbitrary(u)?.mod_floor(initial_balance.atto());
                TokenAmount::from_atto(c)
            } else {
                TokenAmount::from_atto(0)
            };

            let current_balance = initial_balance.clone() - initial_stake.clone();

            let a = StakingAccount {
                public_key: pk,
                secret_key: sk,
                addr,
                initial_balance,
                initial_stake,
                current_balance,
            };
            accounts.push(a);
        }

        // Accounts on the parent subnet.
        let parent_actors = accounts
            .iter()
            .map(|s| Actor {
                meta: ActorMeta::Account(Account {
                    owner: SignerAddr(s.addr),
                }),
                balance: s.current_balance.clone(),
            })
            .collect();

        // Select one validator to be the parent validator, it doesn't matter who.
        let parent_validators = vec![Validator {
            public_key: ValidatorKey(accounts[0].public_key),
            // All the power in the parent subnet belongs to this single validator.
            // We are only interested in the staking of the *child subnet*.
            power: Collateral(TokenAmount::from_whole(1)),
        }];

        // Select some of the accounts to be the initial *child subnet* validators.
        let child_validators = accounts
            .iter()
            .take(num_validators)
            .map(|a| {
                let v = Validator {
                    public_key: ValidatorKey(a.public_key),
                    power: Collateral(a.initial_stake.clone()),
                };
                Ok(v)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // IPC of the parent subnet itself - most are not going to be used.
        let parent_ipc = IpcParams {
            gateway: GatewayParams {
                subnet_id: ArbSubnetID::arbitrary(u)?.0,
                bottom_up_check_period: 1 + u.choose_index(100)? as u64,
                top_down_check_period: 1 + u.choose_index(100)? as u64,
                msg_fee: ArbTokenAmount::arbitrary(u)?.0,
                majority_percentage: 51 + u8::arbitrary(u)? % 50,
                min_collateral: ArbTokenAmount::arbitrary(u)?
                    .0
                    .max(TokenAmount::from_atto(1)),
                active_validators_limit: 1 + u.choose_index(100)? as u16,
            },
        };

        let child_subnet_id = SubnetID::new_from_parent(
            &parent_ipc.gateway.subnet_id,
            ArbSubnetAddress::arbitrary(u)?.0,
        );

        let parent_genesis = Genesis {
            chain_name: String::arbitrary(u)?,
            timestamp: Timestamp(u64::arbitrary(u)?),
            network_version: NetworkVersion::V20,
            base_fee: ArbTokenAmount::arbitrary(u)?.0,
            power_scale: *u.choose(&[0, 3]).expect("non empty"),
            validators: parent_validators,
            accounts: parent_actors,
            ipc: Some(parent_ipc),
        };

        let child_ipc = IpcParams {
            gateway: GatewayParams {
                subnet_id: child_subnet_id,
                bottom_up_check_period: 1 + u.choose_index(100)? as u64,
                top_down_check_period: 1 + u.choose_index(100)? as u64,
                msg_fee: ArbTokenAmount::arbitrary(u)?.0,
                majority_percentage: 51 + u8::arbitrary(u)? % 50,
                min_collateral: ArbTokenAmount::arbitrary(u)?
                    .0
                    .max(TokenAmount::from_atto(1)),
                active_validators_limit: num_max_validators as u16,
            },
        };

        let child_genesis = Genesis {
            chain_name: String::arbitrary(u)?,
            timestamp: Timestamp(u64::arbitrary(u)?),
            network_version: NetworkVersion::V20,
            base_fee: ArbTokenAmount::arbitrary(u)?.0,
            power_scale: *u.choose(&[0, 3]).expect("non empty"),
            validators: child_validators,
            accounts: Vec::new(),
            ipc: Some(child_ipc),
        };

        Ok(StakingState::new(accounts, parent_genesis, child_genesis))
    }
}

enum StakingCommand {
    /// Bottom-up checkpoint; confirms all staking operations up to the configuration number.
    Checkpoint { next_configuration_number: u64 },
    /// Increase the collateral of a validator; when it goes from 0 this means joining the subnet.
    Stake(Address, TokenAmount),
    /// Decrease the collateral of a validator; if it goes to 0 it means leaving the subnet.
    Unstake(Address, TokenAmount),
}

#[derive(Default)]
struct StakingMachine {
    multi_engine: Arc<MultiEngine>,
}

impl StateMachine for StakingMachine {
    type System = StakingSystem;

    type State = StakingState;

    type Command = StakingCommand;

    type Result = ();

    fn gen_state(&self, u: &mut Unstructured) -> arbitrary::Result<Self::State> {
        StakingState::arbitrary(u)
    }

    fn new_system(&self, state: &Self::State) -> Self::System {
        let rt = tokio::runtime::Runtime::new().expect("create tokio runtime for init");

        let (parent_state, _) = rt
            .block_on(contract_test::init_exec_state(
                self.multi_engine.clone(),
                state.parent_genesis.clone(),
            ))
            .expect("failed to init parent");

        let gateway = GatewayCaller::default();

        // TODO: Call the methods on the gateway to establish the subnet based on `state.child_genesis`:
        // * Create the subnet with the given ID
        // * Make all the validators join the subnet by putting down collateral according to their power

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
        let cmd = match u.choose(&["checkpoint", "stake", "unstake"]).unwrap() {
            &"checkpoint" => {
                let cn = match state.pending_updates.len() {
                    0 => state.configuration_number,
                    n => {
                        let idx = u.choose_index(n).expect("non-zero");
                        state.pending_updates[idx].configuration_number
                    }
                };
                StakingCommand::Checkpoint {
                    next_configuration_number: cn,
                }
            }
            &"stake" => {
                let a = u.choose(&state.addrs).expect("accounts not empty");
                let a = state.accounts.get(a).expect("account exists");
                // Limit ourselves to the outstanding balance - the user would not be able to send more value to the contract.
                let b = BigInt::arbitrary(u)?.mod_floor(a.current_balance.atto());
                let b = TokenAmount::from_atto(b);
                StakingCommand::Stake(a.addr, b)
            }
            &"unstake" => {
                let a = u.choose(&state.addrs).expect("accounts not empty");
                let a = state.accounts.get(a).expect("account exists");
                // We can try sending requests to unbond arbitrarily large amounts of collateral - the system should catch any attempt to steal.
                // Only limiting it to be under the initial balance so that it's comparable to what the deposits could have been.
                let b = BigInt::arbitrary(u)?.mod_floor(a.initial_balance.atto());
                let b = TokenAmount::from_atto(b);
                StakingCommand::Unstake(a.addr, b)
            }
            other => unimplemented!("unknown command: {other}"),
        };
        Ok(cmd)
    }

    fn run_command(&self, system: &mut Self::System, cmd: &Self::Command) -> Self::Result {
        // TODO: Execute the command against the contract.
    }

    fn check_result(&self, cmd: &Self::Command, pre_state: &Self::State, result: &Self::Result) {
        // TODO: Check that events emitted by the system are as expected.
    }

    fn next_state(&self, cmd: &Self::Command, state: Self::State) -> Self::State {
        match cmd {
            StakingCommand::Checkpoint {
                next_configuration_number,
            } => state.checkpoint(*next_configuration_number),
            StakingCommand::Stake(addr, value) => state.stake(*addr, value.clone()),
            StakingCommand::Unstake(addr, value) => state.unstake(*addr, value.clone()),
        }
    }

    fn check_system(
        &self,
        cmd: &Self::Command,
        post_state: &Self::State,
        post_system: &Self::System,
    ) {
        match cmd {
            StakingCommand::Checkpoint { .. } => {
                // Sanity check the reference state while we have no contract to compare with.
                debug_assert!(
                    post_state
                        .accounts
                        .iter()
                        .all(|(_, a)| a.current_balance <= a.initial_balance),
                    "no account goes over initial balance"
                );

                debug_assert!(
                    post_state
                        .child_validators
                        .iter()
                        .all(|(_, p)| !p.0.is_zero()),
                    "all child validators have non-zero collateral"
                );
            }
            StakingCommand::Stake(addr, _) | StakingCommand::Unstake(addr, _) => {
                let a = post_state.accounts.get(addr).unwrap();
                debug_assert!(a.current_balance <= a.initial_balance);
            }
        }

        // TODO: Compare the system with the state:
        // * check that balances match
        // * check that active powers match
    }
}

state_machine_test!(staking, 20000 ms, 100 steps, StakingMachine::default());
