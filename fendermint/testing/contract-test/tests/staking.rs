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

use arbitrary::{Arbitrary, Unstructured};
use fendermint_crypto::{PublicKey, SecretKey};
use fendermint_testing::arb::{ArbSubnetAddress, ArbSubnetID, ArbTokenAmount};
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
    child_validators: HashMap<Address, Power>,
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
                (addr, v.power)
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
            let b = ArbTokenAmount::arbitrary(u)?.0;
            let b = b.max(TokenAmount::from_atto(1));
            // Limit the balance to the u64 range, so we don't have to worry about power conversions.
            let b = TokenAmount::from_atto(b.atto() % BigInt::from(u64::MAX));

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
            validators: parent_validators,
            accounts: parent_actors,
            ipc: Some(parent_ipc),
        };

        // Select some of the accounts to be the initial *child subnet* validators.
        let child_validators = accounts
            .iter()
            .take(num_validators)
            .map(|a| {
                // Power has to be a u64.
                let p = BigInt::arbitrary(u)? % a.initial_balance.atto();
                let p = Power(p.try_into().expect("balances are u64"));
                let v = Validator {
                    public_key: ValidatorKey(a.public_key),
                    power: p,
                };
                Ok(v)
            })
            .collect::<Result<Vec<_>, _>>()?;

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
            },
        };

        let child_genesis = Genesis {
            chain_name: String::arbitrary(u)?,
            timestamp: Timestamp(u64::arbitrary(u)?),
            network_version: NetworkVersion::V20,
            base_fee: ArbTokenAmount::arbitrary(u)?.0,
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

struct StakingMachine;

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
            .block_on(contract_test::init_exec_state(state.parent_genesis.clone()))
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
                let b = BigInt::arbitrary(u)? % a.current_balance.atto();
                let b = TokenAmount::from_atto(b);
                StakingCommand::Stake(a.addr, b)
            }
            &"unstake" => {
                let a = u.choose(&state.addrs).expect("accounts not empty");
                let a = state.accounts.get(a).expect("account exists");
                // We can try sending requests to unbond arbitrarily large amounts of collateral - the system should catch any attempt to steal.
                let b = ArbTokenAmount::arbitrary(u)?.0;
                StakingCommand::Stake(a.addr, b)
            }
            other => unimplemented!("unknown command: {other}"),
        };
        Ok(cmd)
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
