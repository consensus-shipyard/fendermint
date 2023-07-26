// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::collections::HashMap;

use anyhow::Context;
use async_trait::async_trait;
use ethers::core::types as et;
use fendermint_vm_actor_interface::eam::EthAddress;
use fendermint_vm_actor_interface::{
    account, burntfunds, cron, eam, init, ipc, reward, system, EMPTY_ARR,
};
use fendermint_vm_core::{chainid, Timestamp};
use fendermint_vm_genesis::{ActorMeta, Genesis, Validator};
use fendermint_vm_ipc_actors::gateway::SubnetID;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::chainid::ChainID;
use fvm_shared::econ::TokenAmount;
use fvm_shared::version::NetworkVersion;
use num_traits::Zero;

use crate::GenesisInterpreter;

use super::state::FvmGenesisState;
use super::FvmMessageInterpreter;

pub struct FvmGenesisOutput {
    pub chain_id: ChainID,
    pub timestamp: Timestamp,
    pub network_version: NetworkVersion,
    pub base_fee: TokenAmount,
    pub circ_supply: TokenAmount,
    pub validators: Vec<Validator>,
}

#[async_trait]
impl<DB> GenesisInterpreter for FvmMessageInterpreter<DB>
where
    DB: Blockstore + 'static + Send + Sync + Clone,
{
    type State = FvmGenesisState<DB>;
    type Genesis = Genesis;
    type Output = FvmGenesisOutput;

    /// Initialize actor states from the Genesis spec.
    ///
    /// This method doesn't create all builtin Filecoin actors,
    /// it leaves out the ones specific to file storage.
    ///
    /// The ones included are:
    /// * system
    /// * init
    /// * cron
    /// * EAM
    ///
    /// TODO:
    /// * burnt funds?
    /// * faucet?
    /// * rewards?
    /// * IPC
    ///
    /// See genesis initialization in:
    /// * [Lotus](https://github.com/filecoin-project/lotus/blob/v1.20.4/chain/gen/genesis/genesis.go)
    /// * [ref-fvm tester](https://github.com/filecoin-project/ref-fvm/blob/fvm%40v3.1.0/testing/integration/src/tester.rs#L99-L103)
    /// * [fvm-workbench](https://github.com/anorth/fvm-workbench/blob/67219b3fd0b5654d54f722ab5acea6ec0abb2edc/builtin/src/genesis.rs)
    async fn init(
        &self,
        mut state: Self::State,
        genesis: Self::Genesis,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        // NOTE: We could consider adding the chain ID to the interpreter
        //       and rejecting genesis if it doesn't match the expectation,
        //       but the Tendermint genesis file also has this field, and
        //       presumably Tendermint checks that its peers have the same.
        let chain_id = chainid::from_str_hashed(&genesis.chain_name)?;

        // Currently we just pass them back as they are, but later we should
        // store them in the IPC actors; or in case of a snapshot restore them
        // from the state.
        let out = FvmGenesisOutput {
            chain_id,
            timestamp: genesis.timestamp,
            network_version: genesis.network_version,
            circ_supply: circ_supply(&genesis),
            base_fee: genesis.base_fee,
            validators: genesis.validators,
        };

        // STAGE 0: Declare the built-in EVM contracts we'll have to deploy.

        let eth_contract_ids = vec![ipc::GATEWAY_ACTOR_ID, ipc::SUBNET_REGISTRY_ACTOR_ID];

        // Collect dependencies of the main IPC actors.
        let eth_libs = self
            .contracts
            .library_dependencies(&[
                ("Gateway.sol", "Gateway"),
                ("SubnetRegistry.sol", "SubnetRegistry"),
            ])
            .context("failed to collect EVM contract dependencies")?;

        // STAGE 1: First we initialize native built-in actors.

        // System actor
        state
            .create_actor(
                system::SYSTEM_ACTOR_CODE_ID,
                system::SYSTEM_ACTOR_ID,
                &system::State {
                    builtin_actors: state.manifest_data_cid,
                },
                TokenAmount::zero(),
                None,
            )
            .context("failed to create system actor")?;

        // Init actor
        let (init_state, addr_to_id) = init::State::new(
            state.store(),
            genesis.chain_name.clone(),
            &genesis.accounts,
            &eth_contract_ids,
            eth_libs.len() as u64,
        )
        .context("failed to create init state")?;

        state
            .create_actor(
                init::INIT_ACTOR_CODE_ID,
                init::INIT_ACTOR_ID,
                &init_state,
                TokenAmount::zero(),
                None,
            )
            .context("failed to create init actor")?;

        // Cron actor
        state
            .create_actor(
                cron::CRON_ACTOR_CODE_ID,
                cron::CRON_ACTOR_ID,
                &cron::State {
                    entries: vec![], // TODO: Maybe with the IPC.
                },
                TokenAmount::zero(),
                None,
            )
            .context("failed to create cron actor")?;

        // Ethereum Account Manager (EAM) actor
        state
            .create_actor(
                eam::EAM_ACTOR_CODE_ID,
                eam::EAM_ACTOR_ID,
                &EMPTY_ARR,
                TokenAmount::zero(),
                None,
            )
            .context("failed to create EAM actor")?;

        // Burnt funds actor (it's just an account).
        state
            .create_actor(
                account::ACCOUNT_ACTOR_CODE_ID,
                burntfunds::BURNT_FUNDS_ACTOR_ID,
                &account::State {
                    address: burntfunds::BURNT_FUNDS_ACTOR_ADDR,
                },
                TokenAmount::zero(),
                None,
            )
            .context("failed to create burnt funds actor")?;

        // A placeholder for the reward actor, beause I don't think
        // using the one in the builtin actors library would be appropriate.
        // This effectively burns the miner rewards. Better than panicking.
        state
            .create_actor(
                account::ACCOUNT_ACTOR_CODE_ID,
                reward::REWARD_ACTOR_ID,
                &account::State {
                    address: reward::REWARD_ACTOR_ADDR,
                },
                TokenAmount::zero(),
                None,
            )
            .context("failed to create reward actor")?;

        // STAGE 2: Create non-builtin accounts which do not have a fixed ID.

        // The next ID is going to be _after_ the accounts, which have already been assigned an ID by the `Init` actor.
        // The reason we aren't using the `init_state.next_id` is because that already accounted for the multisig accounts.
        let mut next_id = init::FIRST_NON_SINGLETON_ADDR + addr_to_id.len() as u64;

        for a in genesis.accounts {
            let balance = a.balance;
            match a.meta {
                ActorMeta::Account(acct) => {
                    state
                        .create_account_actor(acct, balance, &addr_to_id)
                        .context("failed to create account actor")?;
                }
                ActorMeta::Multisig(ms) => {
                    state
                        .create_multisig_actor(ms, balance, &addr_to_id, next_id)
                        .context("failed to create multisig actor")?;
                    next_id += 1;
                }
            }
        }

        // STAGE 3: Initialize the FVM and create built-in FEVM actors.

        state
            .init_exec_state(
                out.timestamp,
                out.network_version,
                out.base_fee.clone(),
                out.circ_supply.clone(),
                out.chain_id.into(),
            )
            .context("failed to init exec state")?;

        // Assign dynamic ID addresses to libraries, but use fixed addresses for the top level contracts.
        let mut eth_lib_addrs = HashMap::new();

        // IPC libraries.
        {
            // Deploy them with non-deterministic IDs.
            for (lib_src, lib_name) in eth_libs {
                let fqn = self.contracts.fqn(&lib_src, &lib_name);
                let bytecode = self
                    .contracts
                    .bytecode(&lib_src, &lib_name, &eth_lib_addrs)
                    .with_context(|| format!("failed to load library contract {fqn}"))?;

                state
                    .create_evm_actor(next_id, bytecode)
                    .with_context(|| format!("failed to create library actor {fqn}"))?;

                eth_lib_addrs.insert(
                    self.contracts.fqn(&lib_src, &lib_name),
                    et::Address::from(EthAddress::from_id(next_id).0),
                );

                next_id += 1;
            }
        }

        // IPC Gateway actor.
        {
            use fendermint_vm_ipc_actors::gateway::GATEWAY_ABI;
            use ipc::gateway::ConstructorParameters;

            let bytecode = self
                .contracts
                .bytecode("Gateway.sol", "Gateway", &eth_lib_addrs)
                .context("failed to load Gateway contract")?;

            // TODO: Move all these parameters to Genesis.
            let params = ConstructorParameters {
                network_name: SubnetID {
                    root: 0,
                    route: Vec::new(),
                },
                bottom_up_check_period: 100,
                top_down_check_period: 100,
                msg_fee: et::U256::from(0),
                majority_percentage: 67,
            };

            state
                .create_evm_actor_with_cons(ipc::GATEWAY_ACTOR_ID, &GATEWAY_ABI, bytecode, params)
                .context("failed to create Gateway actor")?;
        }

        Ok((state, out))
    }
}

/// Sum of balances in the genesis accounts.
fn circ_supply(g: &Genesis) -> TokenAmount {
    g.accounts
        .iter()
        .fold(TokenAmount::zero(), |s, a| s + a.balance.clone())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fendermint_vm_genesis::Genesis;
    use fvm::engine::MultiEngine;
    use quickcheck::Arbitrary;

    use crate::{
        fvm::{
            bundle::{bundle_path, contracts_path},
            store::memory::MemoryBlockstore,
            FvmMessageInterpreter,
        },
        GenesisInterpreter,
    };

    use super::FvmGenesisState;

    #[tokio::test]
    async fn load_genesis() {
        let mut g = quickcheck::Gen::new(5);
        let genesis = Genesis::arbitrary(&mut g);
        let bundle = std::fs::read(bundle_path()).expect("failed to read bundle");
        let store = MemoryBlockstore::new();
        let multi_engine = Arc::new(MultiEngine::default());

        let state = FvmGenesisState::new(store, multi_engine, &bundle)
            .await
            .expect("failed to create state");

        let interpreter = FvmMessageInterpreter::new(contracts_path());

        let (state, out) = interpreter
            .init(state, genesis.clone())
            .await
            .expect("failed to create actors");

        let _state_root = state.commit().expect("failed to commit");
        assert_eq!(out.validators, genesis.validators);
    }
}
